//! Turns a scheduler outcome into what the user perceives: one sound, one notification,
//! a refreshed widget and (optionally) the widget brought forward.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType};
use tauri_plugin_notification::NotificationExt;

use crate::audio::ToneKind;
use crate::domain::ActiveAlert;
use crate::scheduler::TickOutcome;
use crate::state::AppState;

pub const EV_SNAPSHOT: &str = "snapshot-changed";
pub const EV_ALERT: &str = "alert-fired";
pub const EV_ISSUE: &str = "app-issue";
pub const EV_FOCUS_QUICK_ADD: &str = "focus-quick-add";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertFired {
    pub alerts: Vec<ActiveAlert>,
    pub played_sound: bool,
    pub count: usize,
}

pub fn emit_snapshot(app: &AppHandle) {
    let state = app.state::<AppState>();
    let snap = state.snapshot();
    crate::tray::reflect_alerts(app, !snap.alerts.is_empty());
    if let Err(e) = app.emit(EV_SNAPSHOT, snap) {
        log::warn!("emit snapshot failed: {e}");
    }
}

pub fn report_issue(app: &AppHandle, code: &str, message: impl Into<String>) {
    let message = message.into();
    log::warn!("{code}: {message}");
    let state = app.state::<AppState>();
    state.add_issue(code, message.clone());
    let _ = app.emit(EV_ISSUE, crate::state::Issue { code: code.into(), message });
}

pub fn dispatch(app: &AppHandle, outcome: TickOutcome, now: i64) {
    let state = app.state::<AppState>();
    let settings = state.settings();

    let mut played = false;
    if settings.sound_enabled {
        let kind = if outcome.has_due() { ToneKind::Due } else { ToneKind::Lead };
        match state.audio.play(settings.sound_name, kind, settings.sound_volume) {
            Ok(()) => {
                played = true;
                state.clear_issue("audio");
            }
            Err(e) => report_issue(app, "audio", format!("Couldn't play the alert sound: {e}. Check your output device.")),
        }
    }

    if let Some((title, body)) = outcome.notification_text(now) {
        if let Err(e) = app.notification().builder().title(title).body(body).show() {
            report_issue(
                app,
                "notification",
                format!("Desktop notifications aren't available: {e}. Alerts still appear in the widget."),
            );
        }
    }

    if settings.bring_forward_on_alert && outcome.has_due() {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.request_user_attention(Some(UserAttentionType::Informational));
        }
    }

    emit_snapshot(app);
    let alerts = state.core().and_then(|c| c.db.read(crate::db::repo::active_alerts)).unwrap_or_default();
    let _ = app.emit(EV_ALERT, AlertFired { alerts, played_sound: played, count: outcome.fired.len() });
}
