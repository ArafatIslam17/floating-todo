const COMMANDS: &[&str] = &[
    "get_snapshot",
    "create_task",
    "update_task",
    "set_task_completed",
    "delete_task",
    "restore_task",
    "snooze_alert",
    "dismiss_alert",
    "update_settings",
    "test_sound",
    "set_compact",
    "hide_widget",
    "quit_app",
    "retry_startup",
];

fn main() {
    // An app manifest makes every command deny-by-default unless a capability allows it.
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
