//! Configurable global shortcut that shows the widget and focuses quick-add.

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AppError, AppResult};

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                crate::window::show_and_focus(app);
                let _ = app.emit(crate::alerts::EV_FOCUS_QUICK_ADD, ());
            }
        })
        .build()
}

pub fn parse(accelerator: &str) -> AppResult<Shortcut> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| AppError::Shortcut(format!("\"{accelerator}\" isn't a valid shortcut ({e}). Try something like Ctrl+Alt+Space.")))
}

pub fn register(app: &AppHandle, accelerator: &str) -> AppResult<()> {
    let sc = parse(accelerator)?;
    app.global_shortcut().register(sc).map_err(|e| {
        AppError::Shortcut(format!("{accelerator} couldn't be registered — another app may be using it ({e}). Choose a different shortcut."))
    })
}

/// Switch from `old` to `new`; on failure the old binding stays active.
pub fn replace(app: &AppHandle, old: &str, new: &str) -> AppResult<()> {
    let new_sc = parse(new)?;
    if let Ok(old_sc) = parse(old) {
        if old_sc == new_sc {
            return Ok(());
        }
        let _ = app.global_shortcut().unregister(old_sc);
    }
    if let Err(e) = register(app, new) {
        let _ = register(app, old);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_default_and_rejects_garbage() {
        assert!(parse("Ctrl+Alt+Space").is_ok());
        assert!(parse("CommandOrControl+Shift+K").is_ok());
        assert!(parse("Ctrl+Banana").is_err());
        assert!(parse("").is_err());
    }
}
