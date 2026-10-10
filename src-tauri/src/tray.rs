//! System tray icon and menu.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::state::AppState;

const TRAY_ID: &str = "main-tray";
static SHOWING_ALERT_ICON: AtomicBool = AtomicBool::new(false);

fn normal_icon() -> Image<'static> {
    Image::from_bytes(include_bytes!("../icons/tray.png")).expect("tray icon is a valid PNG")
}

fn alert_icon() -> Image<'static> {
    Image::from_bytes(include_bytes!("../icons/tray-alert.png")).expect("tray alert icon is a valid PNG")
}

pub struct TrayMenu {
    pub always_on_top: CheckMenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let name = app.package_info().name.clone();
    let show = MenuItem::with_id(app, "show", "Show widget", true, None::<&str>)?;
    let new_task = MenuItem::with_id(app, "new", "New task", true, None::<&str>)?;
    let on_top = CheckMenuItem::with_id(
        app,
        "on_top",
        "Always on top",
        true,
        app.state::<AppState>().settings().always_on_top,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", format!("Quit {name}"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &new_task, &on_top, &sep, &quit])?;
    app.manage(TrayMenu { always_on_top: on_top.clone() });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(normal_icon())
        .tooltip(&name)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => crate::window::show_and_focus(app),
            "new" => {
                crate::window::show_and_focus(app);
                let _ = app.emit(crate::alerts::EV_FOCUS_QUICK_ADD, ());
            }
            "on_top" => {
                let current = app.state::<AppState>().settings().always_on_top;
                let patch = crate::settings::SettingsPatch { always_on_top: Some(!current), ..Default::default() };
                if let Err(e) = crate::commands::apply_settings_patch(app, patch) {
                    crate::alerts::report_issue(app, "window", e.to_string());
                }
            }
            "quit" => crate::quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::window::toggle(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Swap the tray icon when alerts are active, and keep the tooltip informative.
pub fn reflect_alerts(app: &AppHandle, active: bool) {
    if SHOWING_ALERT_ICON.swap(active, Ordering::SeqCst) == active {
        return;
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let name = app.package_info().name.clone();
        let _ = tray.set_icon(Some(if active { alert_icon() } else { normal_icon() }));
        let _ = tray.set_tooltip(Some(if active { format!("{name}: tasks need attention") } else { name }));
    }
}

pub fn sync_always_on_top(app: &AppHandle, value: bool) {
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu.always_on_top.set_checked(value);
    }
}
