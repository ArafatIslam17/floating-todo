//! IPC boundary. Every command validates input, does its work in one transaction,
//! wakes the scheduler and pushes a fresh snapshot. Commands are `async` so they run
//! on Tauri's worker pool rather than blocking the window's event loop.

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::alerts::emit_snapshot;
use crate::audio::ToneKind;
use crate::db::repo;
use crate::domain::validate::{validate_id, validate_snooze, validate_task};
use crate::domain::{Task, TaskInput};
use crate::error::{AppError, AppResult};
use crate::settings::{Settings, SettingsPatch, SoundName};
use crate::state::{AppState, Snapshot};

fn after_change(app: &AppHandle, stop_audio: bool) {
    let state = app.state::<AppState>();
    if let Ok(core) = state.core() {
        core.scheduler.poke();
    }
    if stop_audio {
        state.audio.stop();
    }
    emit_snapshot(app);
}

#[tauri::command]
pub async fn get_snapshot(app: AppHandle) -> AppResult<Snapshot> {
    Ok(app.state::<AppState>().snapshot())
}

#[tauri::command]
pub async fn create_task(app: AppHandle, input: TaskInput) -> AppResult<Task> {
    let valid = validate_task(input)?;
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    let id = uuid::Uuid::new_v4().to_string();
    let task = core.db.write(|tx| repo::create_task(tx, &id, &valid, now))?;
    after_change(&app, false);
    Ok(task)
}

#[tauri::command]
pub async fn update_task(app: AppHandle, id: String, input: TaskInput) -> AppResult<Task> {
    validate_id(&id)?;
    let valid = validate_task(input)?;
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    let (task, timing_changed) = core.db.write(|tx| repo::update_task(tx, &id, &valid, now))?;
    after_change(&app, timing_changed);
    Ok(task)
}

#[tauri::command]
pub async fn set_task_completed(app: AppHandle, id: String, completed: bool) -> AppResult<Task> {
    validate_id(&id)?;
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    let task = core.db.write(|tx| repo::set_completed(tx, &id, completed, now))?;
    after_change(&app, completed);
    Ok(task)
}

#[tauri::command]
pub async fn delete_task(app: AppHandle, id: String) -> AppResult<()> {
    validate_id(&id)?;
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    core.db.write(|tx| repo::soft_delete(tx, &id, now))?;
    after_change(&app, true);
    Ok(())
}

#[tauri::command]
pub async fn restore_task(app: AppHandle, id: String) -> AppResult<Task> {
    validate_id(&id)?;
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    let task = core.db.write(|tx| repo::restore(tx, &id, now))?;
    after_change(&app, false);
    Ok(task)
}

#[tauri::command]
pub async fn snooze_alert(app: AppHandle, reminder_id: i64, minutes: i64) -> AppResult<i64> {
    let minutes = validate_snooze(minutes)?;
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    let until = core.db.write(|tx| repo::snooze(tx, reminder_id, minutes, now))?;
    after_change(&app, true);
    Ok(until)
}

#[tauri::command]
pub async fn dismiss_alert(app: AppHandle, reminder_id: i64) -> AppResult<()> {
    let state = app.state::<AppState>();
    let core = state.core()?;
    let now = state.now();
    core.db.write(|tx| repo::acknowledge(tx, reminder_id, now))?;
    after_change(&app, true);
    Ok(())
}

/// Shared by the command and the tray menu.
pub fn apply_settings_patch(app: &AppHandle, patch: SettingsPatch) -> AppResult<Settings> {
    let state = app.state::<AppState>();
    let core = state.core()?;
    let old = state.settings();
    let (new, changed) = old.apply_patch(&patch)?;
    if changed.is_empty() {
        return Ok(old);
    }

    // OS-level side effects first; abort without saving if the OS refuses.
    if new.global_shortcut != old.global_shortcut {
        crate::shortcut::replace(app, &old.global_shortcut, &new.global_shortcut)?;
        state.clear_issue("shortcut");
    }
    if new.launch_at_login != old.launch_at_login {
        let al = app.autolaunch();
        let res = if new.launch_at_login { al.enable() } else { al.disable() };
        res.map_err(|e| AppError::Window(format!("Couldn't change launch at login: {e}")))?;
    }

    let now = state.now();
    core.db.write(|tx| repo::save_settings(tx, &changed, now))?;
    {
        let mut s = state.settings.write().map_err(|_| AppError::Internal("settings lock".into()))?;
        let geometry = s.window_geometry;
        *s = new.clone();
        s.window_geometry = geometry;
    }

    if new.always_on_top != old.always_on_top || new.acrylic != old.acrylic {
        for p in crate::window::apply_window_settings(app, &new) {
            crate::alerts::report_issue(app, "window", p);
        }
        crate::tray::sync_always_on_top(app, new.always_on_top);
    }
    if !new.sound_enabled {
        state.audio.stop();
    }
    core.scheduler.poke(); // repeat policy may have changed
    emit_snapshot(app);
    Ok(state.settings())
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, patch: SettingsPatch) -> AppResult<Settings> {
    apply_settings_patch(&app, patch)
}

#[tauri::command]
pub async fn test_sound(app: AppHandle, sound: Option<SoundName>, volume: Option<f64>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let s = state.settings();
    let volume = volume.unwrap_or(s.sound_volume);
    if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
        return Err(AppError::validation("Volume must be between 0 and 1."));
    }
    state
        .audio
        .play(sound.unwrap_or(s.sound_name), ToneKind::Due, volume)
        .map_err(|e| AppError::Audio(format!("Couldn't play the sound: {e}. Check your output device.")))?;
    state.clear_issue("audio");
    Ok(())
}

#[tauri::command]
pub async fn set_compact(app: AppHandle, compact: bool, height: Option<f64>) -> AppResult<()> {
    if let Some(h) = height {
        if !h.is_finite() || !(60.0..=900.0).contains(&h) {
            return Err(AppError::validation("Invalid compact height."));
        }
    }
    let was = app.state::<AppState>().settings().compact;
    crate::window::apply_compact(&app, compact, height)?;
    if was != compact {
        apply_settings_patch(&app, SettingsPatch { compact: Some(compact), ..Default::default() })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_widget(app: AppHandle) -> AppResult<()> {
    crate::window::hide(&app);
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> AppResult<()> {
    crate::quit(&app);
    Ok(())
}

#[tauri::command]
pub async fn retry_startup(app: AppHandle) -> AppResult<Snapshot> {
    crate::init_core(&app);
    emit_snapshot(&app);
    Ok(app.state::<AppState>().snapshot())
}
