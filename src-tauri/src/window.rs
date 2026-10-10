//! Widget window: geometry restore/persist, compact mode, live window settings.

use std::time::{Duration, Instant};

use tauri::utils::config::{WindowEffectsConfig};
use tauri::window::Effect;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::settings::{Settings, WindowGeometry};
use crate::state::AppState;

pub const MAIN: &str = "main";
const SAVE_THROTTLE: Duration = Duration::from_millis(500);
const MARGIN_LOGICAL: f64 = 24.0;

pub fn main_window(app: &AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(MAIN).ok_or_else(|| AppError::Window("widget window not found".into()))
}

/// True if at least 64×32 px of the rectangle lies on some monitor.
pub fn rect_visible(g: &WindowGeometry, monitors: &[(i32, i32, u32, u32)]) -> bool {
    monitors.iter().any(|&(mx, my, mw, mh)| {
        let ix = (g.x + g.width as i32).min(mx + mw as i32) - g.x.max(mx);
        let iy = (g.y + g.height as i32).min(my + mh as i32) - g.y.max(my);
        ix >= 64 && iy >= 32
    })
}

/// Place the window from saved geometry, or bottom-right of the primary work area.
pub fn restore_geometry(window: &WebviewWindow, saved: Option<WindowGeometry>) {
    let monitors: Vec<(i32, i32, u32, u32)> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (m.position().x, m.position().y, m.size().width, m.size().height))
        .collect();
    if let Some(g) = saved.filter(|g| rect_visible(g, &monitors)) {
        let _ = window.set_size(PhysicalSize::new(g.width, g.height));
        let _ = window.set_position(PhysicalPosition::new(g.x, g.y));
        return;
    }
    if let Ok(Some(m)) = window.primary_monitor() {
        let wa = m.work_area();
        let scale = m.scale_factor();
        // A hidden window can report a 0/1-pixel size on some platforms (GTK); fall back to the
        // configured logical size.
        let size = window
            .outer_size()
            .ok()
            .filter(|s| s.width >= 100 && s.height >= 100)
            .unwrap_or(PhysicalSize::new((340.0 * scale) as u32, (460.0 * scale) as u32));
        let margin = (MARGIN_LOGICAL * scale) as i32;
        let x = wa.position.x + wa.size.width as i32 - size.width as i32 - margin;
        let y = wa.position.y + wa.size.height as i32 - size.height as i32 - margin;
        let _ = window.set_position(PhysicalPosition::new(x.max(wa.position.x), y.max(wa.position.y)));
    }
}

/// Called on Moved/Resized. Keeps the latest geometry in memory; persists at most every 500 ms.
pub fn record_geometry(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN) else { return };
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) else { return };
    let state = app.state::<AppState>();
    let compact = state.settings().compact;
    let save_now = {
        let Ok(mut gs) = state.geometry.lock() else { return };
        let expanded_height = match (compact, gs.current) {
            (true, Some(prev)) => prev.expanded_height,
            (true, None) => 460,
            (false, _) => size.height,
        };
        gs.current = Some(WindowGeometry { x: pos.x, y: pos.y, width: size.width, height: size.height, expanded_height });
        gs.dirty = true;
        gs.last_saved.is_none_or(|t| t.elapsed() >= SAVE_THROTTLE)
    };
    if save_now {
        flush_geometry(app);
    }
}

/// Persist pending geometry (also called on hide and exit).
pub fn flush_geometry(app: &AppHandle) {
    let state = app.state::<AppState>();
    let geom = {
        let Ok(mut gs) = state.geometry.lock() else { return };
        if !gs.dirty {
            return;
        }
        gs.dirty = false;
        gs.last_saved = Some(Instant::now());
        gs.current
    };
    let Some(g) = geom else { return };
    if let Ok(mut s) = state.settings.write() {
        s.window_geometry = Some(g);
    }
    if let Ok(core) = state.core() {
        let now = state.now();
        let value = serde_json::to_value(g).expect("geometry serialises");
        if let Err(e) = core.db.write(|tx| repo::save_settings(tx, &[("window_geometry", value)], now)) {
            log::warn!("could not save window position: {e}");
        }
    }
}

/// Resize for compact/expanded mode. `compact_height` is the logical height the UI measured.
pub fn apply_compact(app: &AppHandle, compact: bool, compact_height: Option<f64>) -> AppResult<()> {
    let window = main_window(app)?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.inner_size()?;
    let state = app.state::<AppState>();
    if compact {
        {
            let mut gs = state.geometry.lock().map_err(|_| AppError::Internal("geometry lock".into()))?;
            if !state.settings().compact || gs.current.is_none() {
                let pos = window.outer_position()?;
                gs.current = Some(WindowGeometry { x: pos.x, y: pos.y, width: size.width, height: size.height, expanded_height: size.height.max(200) });
            }
        }
        let h = compact_height.unwrap_or(112.0).clamp(96.0, 400.0);
        window.set_size(PhysicalSize::new(size.width, (h * scale).round() as u32))?;
    } else {
        let expanded = state
            .geometry
            .lock()
            .ok()
            .and_then(|g| g.current.map(|c| c.expanded_height))
            .unwrap_or((460.0 * scale) as u32);
        window.set_size(PhysicalSize::new(size.width, expanded.max((200.0 * scale) as u32)))?;
    }
    Ok(())
}

/// Apply window-level settings. Returns an error message for anything the OS refused.
pub fn apply_window_settings(app: &AppHandle, s: &Settings) -> Vec<String> {
    let mut problems = Vec::new();
    let Ok(window) = main_window(app) else { return problems };
    if let Err(e) = window.set_always_on_top(s.always_on_top) {
        problems.push(format!("Couldn't change always-on-top: {e}"));
    }
    let effects = if s.acrylic {
        Some(WindowEffectsConfig { effects: vec![Effect::Acrylic], ..Default::default() })
    } else {
        None
    };
    if let Err(e) = window.set_effects(effects) {
        problems.push(format!("Acrylic background isn't supported here: {e}"));
    }
    problems
}

pub fn show_and_focus(app: &AppHandle) {
    if let Ok(w) = main_window(app) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn hide(app: &AppHandle) {
    flush_geometry(app);
    if let Ok(w) = main_window(app) {
        let _ = w.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    if let Ok(w) = main_window(app) {
        if w.is_visible().unwrap_or(false) {
            hide(app);
        } else {
            show_and_focus(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(x: i32, y: i32) -> WindowGeometry {
        WindowGeometry { x, y, width: 340, height: 460, expanded_height: 460 }
    }

    #[test]
    fn geometry_on_a_connected_monitor_is_kept() {
        let monitors = [(0, 0, 1920, 1080)];
        assert!(rect_visible(&g(1500, 500), &monitors));
    }

    #[test]
    fn geometry_on_a_disconnected_monitor_is_rejected() {
        let monitors = [(0, 0, 1920, 1080)];
        assert!(!rect_visible(&g(2500, 300), &monitors));
        assert!(!rect_visible(&g(1900, 300), &monitors), "only a 20px sliver visible");
    }

    #[test]
    fn negative_coordinates_on_a_left_monitor_are_valid() {
        let monitors = [(0, 0, 1920, 1080), (-1280, 0, 1280, 1024)];
        assert!(rect_visible(&g(-900, 200), &monitors));
    }
}
