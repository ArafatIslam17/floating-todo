//! Floating Todo desktop application.

pub mod alerts;
pub mod audio;
pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod scheduler;
pub mod settings;
pub mod shortcut;
pub mod state;
pub mod tray;
pub mod window;

use std::sync::{Arc, Mutex, RwLock};

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use crate::db::{repo, Db, DB_FILE};
use crate::domain::SystemClock;
use crate::scheduler::SchedulerDeps;
use crate::settings::Settings;
use crate::state::{AppState, Core, GeometryState};

/// Soft-deleted tasks are kept this long for undo, then purged at startup.
const PURGE_AFTER_MS: i64 = 24 * 60 * 60 * 1000;

/// Open storage, load settings, reconcile missed reminders and start the scheduler.
/// Safe to call again (Retry) after a failure; does nothing once storage is open.
pub fn init_core(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.core.read().map(|c| c.is_some()).unwrap_or(false) {
        return;
    }
    let db = match Db::open(&state.db_path) {
        Ok(db) => Arc::new(db),
        Err(e) => {
            log::error!("database open failed: {e}");
            if let Ok(mut se) = state.startup_error.lock() {
                *se = Some(e.to_string());
            }
            return;
        }
    };
    if let Ok(mut se) = state.startup_error.lock() {
        *se = None;
    }

    let now = state.now();
    if let Err(e) = db.write(|tx| repo::purge_deleted_before(tx, now - PURGE_AFTER_MS)) {
        log::warn!("purge failed: {e}");
    }
    let (mut settings, warnings) = match db.read(repo::load_settings_map) {
        Ok(map) => Settings::from_stored(&map),
        Err(e) => (Settings::default(), vec![format!("could not read settings: {e}")]),
    };
    for w in warnings {
        log::warn!("{w}");
    }
    // The OS is the source of truth for launch-at-login (the user may change it in Task Manager).
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        settings.launch_at_login = enabled;
    }
    if let Ok(mut s) = state.settings.write() {
        *s = settings;
    }

    let deps = SchedulerDeps { db: db.clone(), clock: state.clock.clone(), settings: state.settings.clone() };
    // Startup reconcile runs synchronously so missed reminders produce one combined alert.
    let dispatch_app = app.clone();
    if let Err(e) = scheduler::run_once(&deps, &|o, now| alerts::dispatch(&dispatch_app, o, now)) {
        alerts::report_issue(app, "scheduler", format!("Couldn't check reminders: {e}"));
    }
    let fire_app = app.clone();
    let err_app = app.clone();
    let (handle, _join) = scheduler::spawn(
        deps,
        move |o, now| alerts::dispatch(&fire_app, o, now),
        move |e| alerts::report_issue(&err_app, "scheduler", format!("Reminder check failed: {e}. Retrying in 30 seconds.")),
    );
    let installed = match state.core.write() {
        Ok(mut core) => {
            *core = Some(Arc::new(Core { db, scheduler: handle }));
            true
        }
        Err(_) => false,
    };
    if !installed {
        log::error!("could not install core: state lock poisoned");
    }
}

/// Graceful shutdown: persist window position, stop the scheduler, checkpoint the database.
pub fn quit(app: &AppHandle) {
    window::flush_geometry(app);
    let state = app.state::<AppState>();
    state.audio.stop();
    if let Ok(core) = state.core() {
        core.scheduler.shutdown();
        core.db.checkpoint();
    }
    app.exit(0);
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let info = app.package_info();
    app.manage(AppState {
        core: RwLock::new(None),
        startup_error: Mutex::new(None),
        settings: Arc::new(RwLock::new(Settings::default())),
        issues: Mutex::new(Vec::new()),
        audio: audio::AudioPlayer::spawn(),
        clock: Arc::new(SystemClock),
        db_path: data_dir.join(DB_FILE),
        geometry: Mutex::new(GeometryState::default()),
        product_name: info.name.clone(),
        version: info.version.to_string(),
    });

    init_core(app);
    let settings = app.state::<AppState>().settings();

    if let Err(e) = tray::create(app) {
        alerts::report_issue(app, "tray", format!("The tray icon couldn't be created: {e}. Use the global shortcut to show the widget."));
    }

    let main = window::main_window(app)?;
    window::restore_geometry(&main, settings.window_geometry);
    if let Some(g) = settings.window_geometry {
        if let Ok(mut gs) = app.state::<AppState>().geometry.lock() {
            gs.current = Some(g);
        }
    }
    for p in window::apply_window_settings(app, &settings) {
        alerts::report_issue(app, "window", p);
    }
    main.show()?;

    if let Err(e) = shortcut::register(app, &settings.global_shortcut) {
        alerts::report_issue(app, "shortcut", e.to_string());
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Must be first: a second launch focuses this instance instead of starting a second scheduler.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| window::show_and_focus(app)))
        .plugin(tauri_plugin_notification::init())
        .plugin(shortcut::plugin())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::create_task,
            commands::update_task,
            commands::set_task_completed,
            commands::delete_task,
            commands::restore_task,
            commands::snooze_alert,
            commands::dismiss_alert,
            commands::update_settings,
            commands::test_sound,
            commands::set_compact,
            commands::hide_widget,
            commands::quit_app,
            commands::retry_startup,
        ])
        .setup(|app| setup(app.handle()))
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // Closing the widget hides it to the tray; Quit is in the tray menu and Settings.
                api.prevent_close();
                window::hide(window.app_handle());
            }
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => window::record_geometry(window.app_handle()),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building the application");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, code, .. } = event {
            // Keep running in the tray unless quit explicitly (which passes an exit code).
            if code.is_none() {
                api.prevent_exit();
            } else {
                window::flush_geometry(app);
            }
        }
    });
}
