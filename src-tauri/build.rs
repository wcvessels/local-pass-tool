use std::path::PathBuf;

const COMMANDS: &[&str] = &[
    "generate_passwords",
    "copy_password",
    "clear_sensitive_state",
    "clipboard_status",
    "set_clipboard_timeout",
    "set_window_view",
    "set_macos_best_effort_clear",
    "set_pointer_inside",
    "set_always_on_top",
    "start_window_drag",
    "open_about",
    "close_about",
    "redaction_ack",
    "request_close",
];

fn main() {
    let manifest = tauri_build::AppManifest::new().commands(COMMANDS);
    let attributes = tauri_build::Attributes::new()
        .app_manifest(manifest)
        .windows_attributes(
            tauri_build::WindowsAttributes::new().window_icon_path(PathBuf::from("icons/icon.ico")),
        );

    tauri_build::try_build(attributes).expect("failed to build Tauri context");
}
