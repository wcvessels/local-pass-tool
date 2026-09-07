#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod generator;
mod window_service;

use std::{
    io,
    sync::{Mutex, MutexGuard, mpsc},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent, webview::NewWindowResponse,
};
use window_service::{CloseStart, Effects, RedactionAction, Surface, TimerAction, ViewAction};
use zeroize::{Zeroize, Zeroizing};

const HARDENING_SCRIPT: &str = r#"
(() => {
  'use strict';

  const deny = event => {
    event.preventDefault();
    event.stopImmediatePropagation();
  };
  for (const name of ['copy', 'cut', 'dragstart', 'contextmenu', 'beforeprint']) {
    addEventListener(name, deny, true);
  }
  const blockBrowserShortcut = event => {
    const key = event.key.toLowerCase();
    const modifier = event.ctrlKey || event.metaKey;
    const browserCommand = event.key === 'F5'
      || (modifier && ['p', 's', 'u', 'r', 'o', 'n', 't'].includes(key));
    const devtoolsCommand = event.key === 'F12'
      || (modifier && (event.shiftKey || event.altKey)
        && ['i', 'j', 'c', 'k'].includes(key));
    if (browserCommand || devtoolsCommand) {
      deny(event);
    }
  };
  addEventListener('keydown', blockBrowserShortcut, true);

  const fixedValue = (target, name, value) => {
    try {
      Object.defineProperty(target, name, {
        configurable: false,
        enumerable: false,
        writable: false,
        value
      });
    } catch (_) {}
  };
  const fixedGetter = (target, name, value) => {
    try {
      Object.defineProperty(target, name, {
        configurable: false,
        enumerable: false,
        get: () => value
      });
    } catch (_) {}
  };

  const denyExecCommand = () => false;
  fixedValue(document, 'execCommand', denyExecCommand);
  fixedValue(Document.prototype, 'execCommand', denyExecCommand);
  fixedGetter(navigator, 'clipboard', undefined);
  fixedGetter(Navigator.prototype, 'clipboard', undefined);
  fixedGetter(navigator, 'serviceWorker', undefined);
  fixedValue(window, 'open', () => null);
  fixedValue(window, 'print', () => undefined);
  fixedValue(Window.prototype, 'print', () => undefined);

  const hardeningReady = navigator.clipboard === undefined
    && navigator.serviceWorker === undefined
    && document.execCommand === denyExecCommand
    && Document.prototype.execCommand === denyExecCommand;
  if (!hardeningReady) {
    try {
      Object.defineProperty(window, '__LOCALPASS_HARDENING_FAILED__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: true
      });
    } catch (_) {}
    const failClosed = () => {
      try { document.documentElement.replaceChildren(); } catch (_) {}
    };
    document.addEventListener('DOMContentLoaded', failClosed, { capture: true, once: true });
    failClosed();
  }
})();
"#;
const MASK_ALL_SCRIPT: &str = "window.dispatchEvent(new CustomEvent('localpass:mask-all'));";
const CLIPBOARD_STATUS_CHANGED_SCRIPT: &str =
    "window.dispatchEvent(new CustomEvent('localpass:clipboard-status-changed'));";
const VIEW_ROLLED_SCRIPT: &str =
    "window.dispatchEvent(new CustomEvent('localpass:window-view',{detail:'rolled'}));";
const OPACITY_ACTIVE_SCRIPT: &str =
    "window.dispatchEvent(new CustomEvent('localpass:window-opacity',{detail:'active'}));";
const OPACITY_HOVERED_SCRIPT: &str =
    "window.dispatchEvent(new CustomEvent('localpass:window-opacity',{detail:'hovered'}));";
const OPACITY_IDLE_SCRIPT: &str =
    "window.dispatchEvent(new CustomEvent('localpass:window-opacity',{detail:'idle'}));";
const THEME_DARK_SCRIPT: &str = "document.documentElement.dataset.theme='dark';";
const THEME_LIGHT_SCRIPT: &str = "document.documentElement.dataset.theme='light';";
const WINDOW_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum CommandError {
    Unsupported,
    UnauthorizedWindow,
    InvalidCount,
    InvalidLength,
    NoCharacterGroups,
    EntropyUnavailable,
    StateUnavailable,
    BatchIdExhausted,
    StaleBatch,
    RowOutOfRange,
    StaleViewEpoch,
    InvalidTimeout,
    InvalidCollapsedOpacity,
    ClipboardUnavailable,
    ClipboardCopyFailed,
    ClipboardDestructiveFailure,
    ClipboardCommitUncertain,
    ClipboardReleaseFailed,
    InvalidWindowGeometry,
    WindowUnavailable,
    StaleCloseTicket,
    CloseInProgress,
}

type CommandResult = Result<(), CommandError>;

impl From<generator::GenerateError> for CommandError {
    fn from(error: generator::GenerateError) -> Self {
        match error {
            generator::GenerateError::CountOutOfRange { .. } => Self::InvalidCount,
            generator::GenerateError::LengthOutOfRange { .. } => Self::InvalidLength,
            generator::GenerateError::NoCharacterGroups => Self::NoCharacterGroups,
            generator::GenerateError::Entropy(_) => Self::EntropyUnavailable,
        }
    }
}

impl From<clipboard::ClipboardError> for CommandError {
    fn from(error: clipboard::ClipboardError) -> Self {
        match error {
            clipboard::ClipboardError::ActorUnavailable => Self::ClipboardUnavailable,
            clipboard::ClipboardError::InvalidTimeout => Self::InvalidTimeout,
            clipboard::ClipboardError::Unsupported => Self::Unsupported,
            clipboard::ClipboardError::PreCommitFailure => Self::ClipboardCopyFailed,
            clipboard::ClipboardError::DestructiveFailure => Self::ClipboardDestructiveFailure,
            clipboard::ClipboardError::CommitUncertain => Self::ClipboardCommitUncertain,
            clipboard::ClipboardError::StaleGeneration => Self::StaleBatch,
            clipboard::ClipboardError::ReleaseFailed => Self::ClipboardReleaseFailed,
        }
    }
}
impl From<window_service::WindowServiceError> for CommandError {
    fn from(error: window_service::WindowServiceError) -> Self {
        match error {
            window_service::WindowServiceError::InvalidContentHeight
            | window_service::WindowServiceError::InvalidGeometry => Self::InvalidWindowGeometry,
            window_service::WindowServiceError::UnexpectedWindowLabel { .. } => {
                Self::UnauthorizedWindow
            }
            window_service::WindowServiceError::MonitorUnavailable
            | window_service::WindowServiceError::Native(_) => Self::WindowUnavailable,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    count: usize,
    length: usize,
    lowercase: bool,
    uppercase: bool,
    numbers: bool,
    symbols: bool,
    exclude_ambiguous: bool,
    redacted_view_epoch: u64,
}

#[derive(Debug, Serialize)]
struct GeneratedBatch {
    batch_id: String,
    passwords: Vec<String>,
    view_epoch: u64,
}

impl Drop for GeneratedBatch {
    fn drop(&mut self) {
        zeroize_strings(&mut self.passwords);
    }
}

#[derive(Debug, Serialize)]
struct ClipboardStatus {
    state: &'static str,
    policy: &'static str,
    deadline_ms: Option<u64>,
    timeout_seconds: u32,
    macos_best_effort_clear: bool,
    release_sequence: u64,
    last_release: Option<&'static str>,
}

impl From<clipboard::ClipboardStatus> for ClipboardStatus {
    fn from(status: clipboard::ClipboardStatus) -> Self {
        Self {
            state: status.state,
            policy: status.policy,
            deadline_ms: status.deadline_ms,
            timeout_seconds: status.timeout_seconds,
            macos_best_effort_clear: status.macos_best_effort_clear,
            release_sequence: status.release_sequence,
            last_release: status.last_release.map(clipboard::ReleaseOutcome::as_str),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ClearReason {
    User,
    Regenerate,
    Close,
    SessionEnding,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WindowView {
    Expanded,
    Rolled,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionTheme {
    Dark,
    Light,
}

impl SessionTheme {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    const fn script(self) -> &'static str {
        match self {
            Self::Dark => THEME_DARK_SCRIPT,
            Self::Light => THEME_LIGHT_SCRIPT,
        }
    }
}

struct StoredBatch {
    id: String,
    generation: u64,
    passwords: Vec<Zeroizing<String>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GenerationTicket {
    view_epoch: u64,
}

struct SensitiveState {
    next_batch_id: u64,
    view_epoch: u64,
    pending_generation_epoch: Option<u64>,
    batch: Option<StoredBatch>,
}

impl Default for SensitiveState {
    fn default() -> Self {
        Self {
            next_batch_id: 1,
            view_epoch: 0,
            pending_generation_epoch: None,
            batch: None,
        }
    }
}

struct WindowState {
    coordinator: window_service::Coordinator,
    close: window_service::CloseReducer,
    expanded_height: f64,
    zoom_percent: i32,
    collapsed_opacity_percent: i32,
    about_creating: bool,
    cleanup_epoch: Option<u64>,
    cleanup_deadline: Option<Instant>,
    exit_code: i32,
    cleanup_worker_started: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            coordinator: window_service::Coordinator::default(),
            close: window_service::CloseReducer::default(),
            expanded_height: 640.0,
            zoom_percent: 100,
            collapsed_opacity_percent: 50,
            about_creating: false,
            cleanup_epoch: None,
            cleanup_deadline: None,
            exit_code: 0,
            cleanup_worker_started: false,
        }
    }
}

impl WindowState {
    fn set_collapsed_opacity(
        &mut self,
        percent: i32,
        notify: impl FnOnce(i32) -> CommandResult,
    ) -> Result<i32, CommandError> {
        if self.close.is_closing() {
            return Err(CommandError::CloseInProgress);
        }
        if !(25..=75).contains(&percent) || percent % 5 != 0 {
            return Err(CommandError::InvalidCollapsedOpacity);
        }
        notify(percent)?;
        self.collapsed_opacity_percent = percent;
        Ok(percent)
    }
}

struct AppState {
    sensitive: Mutex<SensitiveState>,
    windows: Mutex<WindowState>,
    clipboard: clipboard::ClipboardActor,
}

#[cfg(test)]
impl Default for AppState {
    fn default() -> Self {
        Self {
            sensitive: Mutex::new(SensitiveState::default()),
            windows: Mutex::new(WindowState::default()),
            clipboard: clipboard::ClipboardActor::for_tests(),
        }
    }
}

impl AppState {
    fn new(app: tauri::AppHandle<tauri::Wry>) -> Self {
        Self {
            sensitive: Mutex::new(SensitiveState::default()),
            windows: Mutex::new(WindowState::default()),
            clipboard: clipboard::ClipboardActor::new(app),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, SensitiveState>, CommandError> {
        self.sensitive
            .lock()
            .map_err(|_| CommandError::StateUnavailable)
    }

    fn lock_windows(&self) -> Result<MutexGuard<'_, WindowState>, CommandError> {
        self.windows
            .lock()
            .map_err(|_| CommandError::StateUnavailable)
    }

    fn begin_generation(&self, view_epoch: u64) -> Result<GenerationTicket, CommandError> {
        let mut state = self.lock()?;
        if view_epoch <= state.view_epoch {
            return Err(CommandError::StaleViewEpoch);
        }

        state.view_epoch = view_epoch;
        state.pending_generation_epoch = Some(view_epoch);
        state.batch = None;

        Ok(GenerationTicket { view_epoch })
    }

    fn abort_generation(&self, ticket: GenerationTicket) -> CommandResult {
        let mut state = self.lock()?;
        if state.pending_generation_epoch == Some(ticket.view_epoch) {
            state.pending_generation_epoch = None;
        }
        Ok(())
    }

    fn commit_generation(
        &self,
        ticket: GenerationTicket,
        passwords: &[String],
    ) -> Result<String, CommandError> {
        let mut state = self.lock()?;
        if state.view_epoch != ticket.view_epoch
            || state.pending_generation_epoch != Some(ticket.view_epoch)
        {
            return Err(CommandError::StaleViewEpoch);
        }

        let id = state.next_batch_id;
        let Some(next_batch_id) = id.checked_add(1) else {
            state.pending_generation_epoch = None;
            return Err(CommandError::BatchIdExhausted);
        };
        state.next_batch_id = next_batch_id;
        state.pending_generation_epoch = None;
        state.batch = Some(StoredBatch {
            id: id.to_string(),
            passwords: passwords.iter().cloned().map(Zeroizing::new).collect(),
            generation: ticket.view_epoch,
        });
        Ok(id.to_string())
    }

    #[cfg(test)]
    fn validate_row(&self, batch_id: &str, row_index: usize) -> CommandResult {
        let state = self.lock()?;
        let batch = state.batch.as_ref().ok_or(CommandError::StaleBatch)?;
        if batch.id != batch_id {
            return Err(CommandError::StaleBatch);
        }
        if row_index >= batch.passwords.len() {
            return Err(CommandError::RowOutOfRange);
        }
        Ok(())
    }

    fn password_for_copy(
        &self,
        batch_id: &str,
        row_index: usize,
    ) -> Result<(Zeroizing<String>, u64), CommandError> {
        let state = self.lock()?;
        let batch = state.batch.as_ref().ok_or(CommandError::StaleBatch)?;
        if batch.id != batch_id {
            return Err(CommandError::StaleBatch);
        }
        let password = batch
            .passwords
            .get(row_index)
            .ok_or(CommandError::RowOutOfRange)?;
        Ok((
            Zeroizing::new(password.as_str().to_owned()),
            batch.generation,
        ))
    }

    fn clear_batch(&self, view_epoch: u64) -> CommandResult {
        let mut state = self.lock()?;
        if view_epoch <= state.view_epoch {
            return Err(CommandError::StaleViewEpoch);
        }
        state.view_epoch = view_epoch;
        state.pending_generation_epoch = None;
        state.batch = None;
        Ok(())
    }

    fn force_clear_for_close(&self) -> u64 {
        let mut state = self
            .sensitive
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let view_epoch = state.view_epoch.saturating_add(1);
        state.view_epoch = view_epoch;
        state.pending_generation_epoch = None;
        state.batch = None;
        view_epoch
    }
}

fn zeroize_strings(passwords: &mut [String]) {
    for password in passwords {
        password.zeroize();
    }
}

fn require_label(window: &WebviewWindow, allowed: &[&str]) -> CommandResult {
    if allowed.contains(&window.label()) {
        Ok(())
    } else {
        Err(CommandError::UnauthorizedWindow)
    }
}

#[cfg(target_os = "windows")]
fn clipboard_owner(window: &WebviewWindow) -> Result<usize, CommandError> {
    let owner = window
        .hwnd()
        .map_err(|_| CommandError::ClipboardUnavailable)?
        .0 as usize;
    if owner == 0 {
        Err(CommandError::ClipboardUnavailable)
    } else {
        Ok(owner)
    }
}

#[cfg(not(target_os = "windows"))]
fn clipboard_owner(_window: &WebviewWindow) -> Result<usize, CommandError> {
    Ok(0)
}

#[tauri::command]
fn generate_passwords(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: GenerateRequest,
) -> Result<GeneratedBatch, CommandError> {
    require_label(&window, &["main"])?;

    let options = generator::GenerationOptions {
        lowercase: request.lowercase,
        uppercase: request.uppercase,
        numbers: request.numbers,
        symbols: request.symbols,
        exclude_ambiguous: request.exclude_ambiguous,
    };
    let ticket = state.begin_generation(request.redacted_view_epoch)?;
    if let Err(error) = state.clipboard.release_async(request.redacted_view_epoch) {
        let _ = state.abort_generation(ticket);
        return Err(error.into());
    }

    let mut passwords = match generator::generate_passwords(request.count, request.length, options)
    {
        Ok(passwords) => passwords,
        Err(error) => {
            let _ = state.abort_generation(ticket);
            return Err(error.into());
        }
    };
    let batch_id = match state.commit_generation(ticket, &passwords) {
        Ok(batch_id) => batch_id,
        Err(error) => {
            zeroize_strings(&mut passwords);
            return Err(error);
        }
    };

    Ok(GeneratedBatch {
        batch_id,
        passwords,
        view_epoch: request.redacted_view_epoch,
    })
}

async fn run_clipboard_operation(
    operation: impl FnOnce() -> Result<clipboard::ClipboardStatus, clipboard::ClipboardError>
    + Send
    + 'static,
) -> Result<ClipboardStatus, CommandError> {
    let status = tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|_| CommandError::ClipboardUnavailable)??;
    Ok(status.into())
}

#[tauri::command]
async fn copy_password(
    window: WebviewWindow,
    state: State<'_, AppState>,
    batch_id: String,
    row_index: usize,
) -> Result<ClipboardStatus, CommandError> {
    require_label(&window, &["main"])?;
    let owner = clipboard_owner(&window)?;
    let (password, generation) = state.password_for_copy(&batch_id, row_index)?;
    let clipboard = state.clipboard.clone();
    run_clipboard_operation(move || clipboard.copy(password, owner, generation)).await
}

#[tauri::command]
async fn clear_sensitive_state(
    window: WebviewWindow,
    state: State<'_, AppState>,
    _reason: ClearReason,
    redacted_view_epoch: u64,
) -> Result<ClipboardStatus, CommandError> {
    require_label(&window, &["main"])?;
    state.clear_batch(redacted_view_epoch)?;
    let clipboard = state.clipboard.clone();
    run_clipboard_operation(move || clipboard.release(redacted_view_epoch)).await
}

#[tauri::command]
async fn clipboard_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<ClipboardStatus, CommandError> {
    require_label(&window, &["main", "about"])?;
    let clipboard = state.clipboard.clone();
    run_clipboard_operation(move || clipboard.status()).await
}

#[tauri::command]
async fn set_clipboard_timeout(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    seconds: u32,
) -> Result<ClipboardStatus, CommandError> {
    require_label(&window, &["about"])?;
    let clipboard = state.clipboard.clone();
    let status = run_clipboard_operation(move || clipboard.set_timeout(seconds)).await?;
    notify_clipboard_status_changed(&app);
    Ok(status)
}

#[tauri::command]
fn set_window_view(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    view: WindowView,
    high_contrast: bool,
    content_height: f64,
    zoom_percent: i32,
) -> CommandResult {
    require_label(&window, &["main"])?;
    window_service::validate_zoom_percent(zoom_percent)?;
    let (native_view, effects) = {
        let mut windows = state.lock_windows()?;
        if windows.close.is_closing() {
            return Err(CommandError::CloseInProgress);
        }
        let effects = windows.coordinator.set_high_contrast(high_contrast);
        let requested = match view {
            WindowView::Expanded => window_service::View::Expanded,
            WindowView::Rolled if windows.coordinator.is_high_contrast() => {
                window_service::View::Expanded
            }
            WindowView::Rolled => window_service::View::Rolled,
        };
        (requested, effects)
    };

    window_service::set_main_view(&window, native_view, content_height, zoom_percent)?;
    let mut windows = state.lock_windows()?;
    windows.zoom_percent = zoom_percent;
    if native_view == window_service::View::Expanded {
        windows.expanded_height = content_height.ceil();
    }
    windows.coordinator.note_view(native_view);
    drop(windows);
    apply_effects(&app, effects);
    Ok(())
}

#[tauri::command]
async fn set_macos_best_effort_clear(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ClipboardStatus, CommandError> {
    require_label(&window, &["about"])?;
    let clipboard = state.clipboard.clone();
    let status = run_clipboard_operation(move || clipboard.set_macos_best_effort(enabled)).await?;
    notify_clipboard_status_changed(&app);
    Ok(status)
}

fn notify_clipboard_status_changed(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.eval(CLIPBOARD_STATUS_CHANGED_SCRIPT);
    }
}

#[tauri::command]
async fn set_collapsed_opacity(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    percent: i32,
) -> Result<i32, CommandError> {
    require_label(&window, &["about"])?;
    let main = app
        .get_webview_window("main")
        .ok_or(CommandError::WindowUnavailable)?;
    state.lock_windows()?.set_collapsed_opacity(percent, |accepted| {
        main.eval(format!(
            "window.dispatchEvent(new CustomEvent('localpass:collapsed-opacity',{{detail:{accepted}}}));"
        ))
        .map_err(|_| CommandError::WindowUnavailable)
    })
}

#[tauri::command]
fn set_pointer_inside(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    inside: bool,
) -> CommandResult {
    let surface = surface_for_window(&window)?;
    let effects = {
        let mut windows = state.lock_windows()?;
        if windows.close.is_closing() {
            return Err(CommandError::CloseInProgress);
        }
        windows.coordinator.set_pointer_inside(surface, inside)
    };
    apply_effects(&app, effects);
    Ok(())
}

#[tauri::command]
fn set_always_on_top(window: WebviewWindow, enabled: bool) -> CommandResult {
    window_service::set_main_topmost(&window, enabled)?;
    Ok(())
}

#[tauri::command]
fn start_window_drag(window: WebviewWindow) -> CommandResult {
    window_service::start_window_drag(&window)?;
    Ok(())
}

#[tauri::command]
async fn open_about(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    theme: SessionTheme,
) -> CommandResult {
    require_label(&window, &["main"])?;
    open_about_window(&app, &state, &window, theme)
}

#[tauri::command]
fn close_about(window: WebviewWindow, app: AppHandle, state: State<'_, AppState>) -> CommandResult {
    require_label(&window, &["about"])?;
    close_about_window(&app, &state)
}

#[tauri::command]
fn redaction_ack(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    close_ticket: u64,
) -> CommandResult {
    require_label(&window, &["main"])?;
    acknowledge_close(&app, &state, close_ticket)
}

#[tauri::command]
fn request_close(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    redacted_view_epoch: u64,
) -> CommandResult {
    require_label(&window, &["main"])?;
    begin_frontend_close(&app, &state, redacted_view_epoch)
}

fn surface_for_window(window: &WebviewWindow) -> Result<Surface, CommandError> {
    match window.label() {
        "main" => Ok(Surface::Main),
        "about" => Ok(Surface::About),
        _ => Err(CommandError::UnauthorizedWindow),
    }
}

fn apply_effects(app: &AppHandle, effects: Effects) {
    let state = app.state::<AppState>();
    let (expanded_height, zoom_percent, closing) = {
        let windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (
            windows.expanded_height,
            windows.zoom_percent,
            windows.close.is_closing(),
        )
    };
    if closing {
        return;
    }

    if let Some(main) = app.get_webview_window("main") {
        match effects.view {
            ViewAction::Roll => {
                let _ = main.eval(VIEW_ROLLED_SCRIPT);
                let _ = window_service::set_main_view(
                    &main,
                    window_service::View::Rolled,
                    expanded_height,
                    zoom_percent,
                );
            }
            ViewAction::Unchanged => {}
        }
    }

    let opacity_script = if effects.opacity >= 1.0 {
        OPACITY_ACTIVE_SCRIPT
    } else if effects.opacity >= 0.95 {
        OPACITY_HOVERED_SCRIPT
    } else {
        OPACITY_IDLE_SCRIPT
    };
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.eval(opacity_script);
    }
    if let Some(about) = app.get_webview_window("about") {
        let _ = about.eval(opacity_script);
    }

    if let TimerAction::Defer { token } = effects.timer {
        let deferred_app = app.clone();
        let callback_app = deferred_app.clone();
        let _ = deferred_app.run_on_main_thread(move || {
            let state = callback_app.state::<AppState>();
            let effects = {
                let mut windows = state
                    .windows
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if windows.close.is_closing() {
                    return;
                }
                windows.coordinator.roll_due(token)
            };
            apply_effects(&callback_app, effects);
        });
    }
}

fn install_window_events(app: &AppHandle, window: &WebviewWindow, surface: Surface) {
    let app = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(focused) => {
            if surface == Surface::About && !*focused {
                let state = app.state::<AppState>();
                let _ = close_about_window(&app, state.inner());
                return;
            }
            if surface == Surface::Main && !*focused {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.eval(MASK_ALL_SCRIPT);
                }
            }
            let state = app.state::<AppState>();
            let effects = {
                let mut windows = state
                    .windows
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                windows.coordinator.set_focus(surface, *focused)
            };
            apply_effects(&app, effects);
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let state = app.state::<AppState>();
            if surface == Surface::Main {
                let _ = begin_native_close(&app, state.inner());
            } else {
                let _ = close_about_window(&app, state.inner());
            }
        }
        WindowEvent::Destroyed if surface == Surface::About => {
            mark_about_closed(&app);
        }
        _ => {}
    });
}

fn open_about_window(
    app: &AppHandle,
    state: &AppState,
    main: &WebviewWindow,
    theme: SessionTheme,
) -> CommandResult {
    if let Some(about) = app.get_webview_window("about") {
        let effects = {
            let mut windows = state.lock_windows()?;
            if windows.close.is_closing() {
                return Err(CommandError::CloseInProgress);
            }
            windows.coordinator.set_about_open(true)
        };
        apply_effects(app, effects);
        about
            .eval(theme.script())
            .map_err(|_| CommandError::WindowUnavailable)?;
        window_service::place_about(main, &about)?;
        about
            .show()
            .and_then(|_| about.set_focus())
            .map_err(|_| CommandError::WindowUnavailable)?;
        return Ok(());
    }

    let (effects, collapsed_opacity_percent) = {
        let mut windows = state.lock_windows()?;
        if windows.close.is_closing() {
            return Err(CommandError::CloseInProgress);
        }
        if windows.about_creating {
            return Err(CommandError::WindowUnavailable);
        }
        windows.about_creating = true;
        (
            windows.coordinator.set_about_open(true),
            windows.collapsed_opacity_percent,
        )
    };
    apply_effects(app, effects);

    let result = (|| -> CommandResult {
        let mut config = app
            .config()
            .app
            .windows
            .iter()
            .find(|window| window.label == "about")
            .cloned()
            .ok_or(CommandError::WindowUnavailable)?;
        config.url = WebviewUrl::App(
            format!(
                "index.html?view=about&theme={}&collapsedOpacityPercent={collapsed_opacity_percent}",
                theme.as_str()
            )
            .into(),
        );

        let builder = WebviewWindowBuilder::from_config(app, &config)
            .map_err(|_| CommandError::WindowUnavailable)?
            .focused(false)
            .parent(main)
            .map_err(|_| CommandError::WindowUnavailable)?;
        let about = builder
            .initialization_script(HARDENING_SCRIPT)
            .on_navigation(allowed_navigation)
            .on_new_window(|_, _| NewWindowResponse::Deny)
            .on_download(|_, _| false)
            .build()
            .map_err(|_| CommandError::WindowUnavailable)?;

        install_window_events(app, &about, Surface::About);
        {
            let mut windows = state.lock_windows()?;
            windows.about_creating = false;
        }
        if let Err(error) = window_service::place_about(main, &about) {
            let _ = about.destroy();
            return Err(error.into());
        }
        if about.show().and_then(|_| about.set_focus()).is_err() {
            let _ = about.destroy();
            return Err(CommandError::WindowUnavailable);
        }
        Ok(())
    })();

    if result.is_err() {
        let effects = {
            let mut windows = state.lock_windows()?;
            windows.about_creating = false;
            windows.coordinator.set_about_open(false)
        };
        apply_effects(app, effects);
    }
    result
}

fn close_about_window(app: &AppHandle, state: &AppState) -> CommandResult {
    let effects = {
        let mut windows = state.lock_windows()?;
        windows.about_creating = false;
        windows.coordinator.set_about_open(false)
    };
    let result = if let Some(about) = app.get_webview_window("about") {
        about.destroy().map_err(|_| CommandError::WindowUnavailable)
    } else {
        Ok(())
    };
    apply_effects(app, effects);
    result
}

fn mark_about_closed(app: &AppHandle) {
    let state = app.state::<AppState>();
    let effects = {
        let mut windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        windows.about_creating = false;
        windows.coordinator.set_about_open(false)
    };
    apply_effects(app, effects);
}

fn begin_frontend_close(
    app: &AppHandle,
    state: &AppState,
    redacted_view_epoch: u64,
) -> CommandResult {
    {
        let mut windows = state.lock_windows()?;
        if windows.close.is_closing() {
            return Err(CommandError::CloseInProgress);
        }
        state.clear_batch(redacted_view_epoch)?;
        if windows.close.begin_frontend() != CloseStart::DestroyNow {
            return Err(CommandError::CloseInProgress);
        }
        windows.cleanup_epoch = Some(redacted_view_epoch);
    }

    start_close_io(app, state, redacted_view_epoch);
    destroy_for_close(app, state)
}

fn begin_native_close(app: &AppHandle, state: &AppState) -> CommandResult {
    let (ticket, after, cleanup_epoch) = {
        let mut windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let CloseStart::AwaitRedaction { ticket, after } = windows.close.begin_native() else {
            return Ok(());
        };
        let cleanup_epoch = state.force_clear_for_close();
        windows.cleanup_epoch = Some(cleanup_epoch);
        (ticket, after, cleanup_epoch)
    };

    start_close_io(app, state, cleanup_epoch);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.eval(native_close_script(ticket));
    }

    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(after);
        redaction_timeout(&app, ticket);
    });
    Ok(())
}

fn start_close_io(app: &AppHandle, state: &AppState, cleanup_epoch: u64) {
    if let Some(about) = app.get_webview_window("about") {
        let _ = about.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }

    let should_spawn = {
        let mut windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if windows.cleanup_worker_started {
            false
        } else {
            windows.cleanup_worker_started = true;
            true
        }
    };
    if !should_spawn {
        return;
    }

    let clipboard = state.clipboard.clone();
    let release_failed = clipboard.release_async(cleanup_epoch).is_err();
    let app = app.clone();
    thread::spawn(move || monitor_close_cleanup(app, clipboard, release_failed));
}

fn native_close_script(ticket: u64) -> String {
    format!(
        "(()=>{{document.querySelectorAll('.password-text').forEach(element=>{{element.textContent='';}});window.dispatchEvent(new CustomEvent('localpass:native-close',{{detail:{ticket}}}));}})();"
    )
}

fn acknowledge_close(app: &AppHandle, state: &AppState, ticket: u64) -> CommandResult {
    let action = {
        let mut windows = state.lock_windows()?;
        windows.close.acknowledge(ticket)
    };
    match action {
        RedactionAction::DestroyNow => destroy_for_close(app, state),
        RedactionAction::Stale | RedactionAction::Unchanged => Err(CommandError::StaleCloseTicket),
    }
}

fn redaction_timeout(app: &AppHandle, ticket: u64) {
    let state = app.state::<AppState>();
    let action = {
        let mut windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        windows.close.redaction_due(ticket)
    };
    if action == RedactionAction::DestroyNow {
        let _ = destroy_for_close(app, state.inner());
    }
}

fn destroy_for_close(app: &AppHandle, state: &AppState) -> CommandResult {
    let should_destroy = {
        let mut windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        windows.close.mark_destroyed()
    };
    if !should_destroy {
        return Ok(());
    }

    let mut failed = false;
    if let Some(about) = app.get_webview_window("about") {
        failed |= about.destroy().is_err();
    }
    if let Some(main) = app.get_webview_window("main") {
        failed |= main.destroy().is_err();
    }
    if failed {
        Err(CommandError::WindowUnavailable)
    } else {
        Ok(())
    }
}

fn monitor_close_cleanup(
    app: AppHandle,
    clipboard: clipboard::ClipboardActor,
    release_failed: bool,
) {
    let (done, completed) = mpsc::channel();
    thread::spawn(move || {
        if !release_failed {
            loop {
                match clipboard.status() {
                    Ok(status) if matches!(status.state, "release_pending" | "countdown") => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    _ => break,
                }
            }
        }
        let _ = done.send(());
    });

    let mut cleanup_complete = release_failed;
    loop {
        if !cleanup_complete {
            cleanup_complete = match completed.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => true,
                Err(mpsc::TryRecvError::Empty) => false,
            };
        }

        let state = app.state::<AppState>();
        let (destroyed, deadline, exit_code) = {
            let windows = state
                .windows
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                windows.close.is_destroyed(),
                windows.cleanup_deadline,
                windows.exit_code,
            )
        };
        let deadline_reached = deadline.is_some_and(|value| Instant::now() >= value);
        if destroyed && (cleanup_complete || deadline_reached) {
            let should_exit = {
                let mut windows = state
                    .windows
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                windows.close.allow_exit()
            };
            if should_exit {
                app.exit(exit_code);
            }
            return;
        }
        thread::sleep(WINDOW_POLL_INTERVAL);
    }
}

fn handle_exit_requested(app: &AppHandle, code: Option<i32>) -> bool {
    let state = app.state::<AppState>();
    let start_close = {
        let mut windows = state
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match windows.close.exit_requested() {
            window_service::ExitAction::Allow => return true,
            window_service::ExitAction::Prevent => return false,
            window_service::ExitAction::StartBoundedClose => {}
        }
        let deadline = Instant::now() + window_service::EXIT_CLEANUP_DELAY;
        windows.cleanup_deadline = Some(
            windows
                .cleanup_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
        if let Some(code) = code {
            windows.exit_code = code;
        }
        true
    };
    if start_close {
        let _ = begin_native_close(app, state.inner());
    }
    false
}
fn allowed_navigation(url: &tauri::webview::Url) -> bool {
    let no_credentials = url.username().is_empty() && url.password().is_none();
    #[cfg(target_os = "windows")]
    let embedded = matches!(
        (url.scheme(), url.host_str(), url.port()),
        ("https", Some("tauri.localhost"), None)
    );
    #[cfg(not(target_os = "windows"))]
    let embedded = matches!(
        (url.scheme(), url.host_str(), url.port()),
        ("tauri", Some("localhost"), None)
    );
    let development = cfg!(debug_assertions)
        && url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port() == Some(1420);
    no_credentials && (embedded || development)
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .invoke_handler(tauri::generate_handler![
            generate_passwords,
            copy_password,
            clear_sensitive_state,
            clipboard_status,
            set_clipboard_timeout,
            set_window_view,
            set_macos_best_effort_clear,
            set_collapsed_opacity,
            set_pointer_inside,
            set_always_on_top,
            start_window_drag,
            open_about,
            close_about,
            redaction_ack,
            request_close
        ])
        .setup(|app| {
            if !app.manage(AppState::new(app.handle().clone())) {
                return Err(io::Error::other("application state already managed").into());
            }

            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or_else(|| io::Error::other("missing main window config"))?;
            let window = WebviewWindowBuilder::from_config(app, &config)?
                .initialization_script(HARDENING_SCRIPT)
                .on_navigation(allowed_navigation)
                .on_new_window(|_, _| NewWindowResponse::Deny)
                .on_download(|_, _| false)
                .build()?;
            install_window_events(app.handle(), &window, Surface::Main);
            window.show()?;
            window.set_focus()?;

            let state = app.state::<AppState>();
            let effects = {
                let mut windows = state
                    .windows
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                windows.coordinator.set_focus(Surface::Main, true)
            };
            apply_effects(app.handle(), effects);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building LocalPass");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if !handle_exit_requested(app, code) {
                api.prevent_exit();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{AppState, CommandError, WindowState, allowed_navigation};

    #[test]
    fn collapsed_opacity_accepts_only_range_and_step_values() {
        let mut windows = WindowState::default();
        assert_eq!(windows.collapsed_opacity_percent, 50);
        for percent in (25..=75).step_by(5) {
            assert_eq!(
                windows
                    .set_collapsed_opacity(percent, |accepted| {
                        assert_eq!(accepted, percent);
                        Ok(())
                    })
                    .unwrap(),
                percent
            );
            assert_eq!(windows.collapsed_opacity_percent, percent);
        }
        for percent in [i32::MIN, -5, 0, 20, 24, 26, 49, 74, 76, 80, i32::MAX] {
            assert!(matches!(
                windows.set_collapsed_opacity(percent, |_| panic!("must not notify")),
                Err(CommandError::InvalidCollapsedOpacity)
            ));
            assert_eq!(windows.collapsed_opacity_percent, 75);
        }
    }

    #[test]
    fn collapsed_opacity_rejection_preserves_session_value() {
        let mut windows = WindowState::default();
        windows.set_collapsed_opacity(25, |_| Ok(())).unwrap();
        assert!(matches!(
            windows.set_collapsed_opacity(75, |_| Err(CommandError::WindowUnavailable)),
            Err(CommandError::WindowUnavailable)
        ));
        assert_eq!(windows.collapsed_opacity_percent, 25);
        windows.close.begin_native();
        assert!(matches!(
            windows.set_collapsed_opacity(75, |_| panic!("must not notify")),
            Err(CommandError::CloseInProgress)
        ));
        assert_eq!(windows.collapsed_opacity_percent, 25);
    }

    #[test]
    fn collapsed_opacity_survives_about_reopen_but_not_a_new_session() {
        let mut windows = WindowState::default();
        windows.coordinator.set_about_open(true);
        windows.set_collapsed_opacity(75, |_| Ok(())).unwrap();
        windows.coordinator.set_about_open(false);
        windows.coordinator.set_about_open(true);
        assert_eq!(windows.collapsed_opacity_percent, 75);
        assert_eq!(WindowState::default().collapsed_opacity_percent, 50);
    }

    #[test]
    fn navigation_is_exact_and_local() {
        assert_eq!(
            allowed_navigation(&"tauri://localhost/index.html".parse().unwrap()),
            !cfg!(target_os = "windows")
        );
        assert_eq!(
            allowed_navigation(&"https://tauri.localhost/index.html".parse().unwrap()),
            cfg!(target_os = "windows")
        );
        assert!(!allowed_navigation(
            &"http://tauri.localhost/index.html".parse().unwrap()
        ));
        assert!(!allowed_navigation(
            &"https://example.com/".parse().unwrap()
        ));
        assert!(!allowed_navigation(
            &"https://tauri.localhost.example.com/".parse().unwrap()
        ));
        assert!(!allowed_navigation(
            &"https://user@tauri.localhost/".parse().unwrap()
        ));
        assert!(!allowed_navigation(
            &"https://tauri.localhost:444/".parse().unwrap()
        ));
        assert_eq!(
            allowed_navigation(&"http://127.0.0.1:1420/".parse().unwrap()),
            cfg!(debug_assertions)
        );
    }

    #[test]
    fn stored_batches_are_opaque_and_replaceable() {
        let state = AppState::default();
        let first_ticket = state.begin_generation(1).unwrap();
        let first = state
            .commit_generation(first_ticket, &["first".to_owned()])
            .unwrap();
        let second_ticket = state.begin_generation(2).unwrap();
        let second = state
            .commit_generation(second_ticket, &["second".to_owned()])
            .unwrap();

        assert_ne!(first, second);
        assert!(matches!(
            state.validate_row(&first, 0),
            Err(CommandError::StaleBatch)
        ));
        state.validate_row(&second, 0).unwrap();
        assert!(matches!(
            state.validate_row(&second, 1),
            Err(CommandError::RowOutOfRange)
        ));
    }

    #[test]
    fn stale_view_epoch_cannot_clear_newer_batch() {
        let state = AppState::default();
        let ticket = state.begin_generation(1).unwrap();
        let id = state
            .commit_generation(ticket, &["secret".to_owned()])
            .unwrap();

        assert!(matches!(
            state.clear_batch(1),
            Err(CommandError::StaleViewEpoch)
        ));
        state.validate_row(&id, 0).unwrap();
        state.clear_batch(2).unwrap();
        assert!(matches!(
            state.validate_row(&id, 0),
            Err(CommandError::StaleBatch)
        ));
    }

    #[test]
    fn begin_generation_invalidates_old_batch_before_work_starts() {
        let state = AppState::default();
        let first_ticket = state.begin_generation(1).unwrap();
        let first = state
            .commit_generation(first_ticket, &["first".to_owned()])
            .unwrap();

        let _second_ticket = state.begin_generation(2).unwrap();

        assert!(matches!(
            state.validate_row(&first, 0),
            Err(CommandError::StaleBatch)
        ));
    }

    #[test]
    fn newer_generation_supersedes_out_of_order_commit() {
        let state = AppState::default();
        let first_ticket = state.begin_generation(1).unwrap();
        let second_ticket = state.begin_generation(2).unwrap();
        let second = state
            .commit_generation(second_ticket, &["second".to_owned()])
            .unwrap();

        assert!(matches!(
            state.commit_generation(first_ticket, &["first".to_owned()]),
            Err(CommandError::StaleViewEpoch)
        ));
        state.validate_row(&second, 0).unwrap();
    }

    #[test]
    fn clear_supersedes_in_flight_generation() {
        let state = AppState::default();
        let ticket = state.begin_generation(1).unwrap();

        state.clear_batch(2).unwrap();

        assert!(matches!(
            state.commit_generation(ticket, &["late".to_owned()]),
            Err(CommandError::StaleViewEpoch)
        ));
    }

    #[test]
    fn epochs_must_increase_but_may_skip_missing_requests() {
        let state = AppState::default();

        let ticket = state.begin_generation(2).unwrap();
        assert!(matches!(
            state.begin_generation(1),
            Err(CommandError::StaleViewEpoch)
        ));
        assert!(matches!(
            state.begin_generation(2),
            Err(CommandError::StaleViewEpoch)
        ));
        state.abort_generation(ticket).unwrap();
        state.clear_batch(4).unwrap();
    }
}
