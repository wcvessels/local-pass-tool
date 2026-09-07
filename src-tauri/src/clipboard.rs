use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use zeroize::Zeroizing;

const DEFAULT_TIMEOUT_SECONDS: u32 = 30;
const BUSY_RETRY: Duration = Duration::from_millis(250);
const UNCERTAIN_MAX_ATTEMPTS: u8 = 8;

#[cfg(any(target_os = "macos", test))]
type MainThreadTask = Box<dyn FnOnce() + Send + 'static>;

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BridgeError {
    ScheduleFailed,
    CompletionLost,
}

#[cfg(any(target_os = "macos", test))]
fn run_scheduled<T, E>(
    schedule: impl FnOnce(MainThreadTask) -> Result<(), E>,
    task: impl FnOnce() -> T + Send + 'static,
) -> Result<T, BridgeError>
where
    T: Send + 'static,
{
    let (reply, response) = mpsc::sync_channel(1);
    let scheduled = Box::new(move || {
        let _ = reply.send(task());
    });
    schedule(scheduled).map_err(|_| BridgeError::ScheduleFailed)?;
    response.recv().map_err(|_| BridgeError::CompletionLost)
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinuxSession {
    X11,
    Wayland,
    Unavailable,
}

#[cfg(any(target_os = "linux", test))]
fn x11_display_is_local(display: Option<&str>) -> bool {
    let Some(display) = display else {
        return false;
    };
    let Some(address) = display
        .strip_prefix(':')
        .or_else(|| display.strip_prefix("unix:"))
        .or_else(|| display.strip_prefix("unix/:"))
    else {
        return false;
    };
    let mut components = address.split('.');
    let Some(display_number) = components.next() else {
        return false;
    };
    let screen_number = components.next();
    !display_number.is_empty()
        && display_number.bytes().all(|byte| byte.is_ascii_digit())
        && screen_number.is_none_or(|screen| {
            !screen.is_empty() && screen.bytes().all(|byte| byte.is_ascii_digit())
        })
        && components.next().is_none()
}

#[cfg(any(target_os = "linux", test))]
fn classify_linux_session(
    wayland_display_present: bool,
    xdg_session_type: Option<&str>,
    local_display_present: bool,
) -> LinuxSession {
    let session_type = xdg_session_type
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if wayland_display_present
        || session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
    {
        return LinuxSession::Wayland;
    }
    if !local_display_present {
        return LinuxSession::Unavailable;
    }
    match session_type {
        None => LinuxSession::X11,
        Some(value) if value.eq_ignore_ascii_case("x11") => LinuxSession::X11,
        Some(_) => LinuxSession::Unavailable,
    }
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy)]
struct X11TargetAtoms {
    targets: u32,
    utf8_string: u32,
    text: u32,
    string: u32,
    text_plain_utf8: u32,
    text_plain: u32,
    kde_hint: u32,
    save_targets: u32,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum X11TargetReply {
    Targets,
    Text(u32),
    Hint,
    Reject,
}

#[cfg(any(target_os = "linux", test))]
impl X11TargetAtoms {
    const fn offered(self) -> [u32; 7] {
        [
            self.targets,
            self.utf8_string,
            self.text,
            self.string,
            self.text_plain_utf8,
            self.text_plain,
            self.kde_hint,
        ]
    }

    fn reply_for(self, target: u32) -> X11TargetReply {
        if target == self.save_targets {
            X11TargetReply::Reject
        } else if target == self.targets {
            X11TargetReply::Targets
        } else if target == self.kde_hint {
            X11TargetReply::Hint
        } else if target == self.text {
            X11TargetReply::Text(self.utf8_string)
        } else if [
            self.utf8_string,
            self.string,
            self.text_plain_utf8,
            self.text_plain,
        ]
        .contains(&target)
        {
            X11TargetReply::Text(target)
        } else {
            X11TargetReply::Reject
        }
    }
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct X11LeaseToken {
    lease_id: u64,
    window: u32,
}

#[cfg(any(target_os = "linux", test))]
fn x11_token_matches(active: Option<X11LeaseToken>, requested: X11LeaseToken) -> bool {
    active == Some(requested)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardError {
    ActorUnavailable,
    InvalidTimeout,
    Unsupported,
    PreCommitFailure,
    DestructiveFailure,
    CommitUncertain,
    StaleGeneration,
    ReleaseFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseOutcome {
    Cleared,
    OwnershipLost,
    Busy,
    Unsupported,
    Fatal,
    SuppressedByPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ClipboardStatus {
    pub state: &'static str,
    pub policy: &'static str,
    pub deadline_ms: Option<u64>,
    pub timeout_seconds: u32,
    pub macos_best_effort_clear: bool,
    pub last_release: Option<ReleaseOutcome>,
    pub release_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClearPolicy {
    OwnershipSafe,
    NonMutating,
    BestEffortArmed,
    BestEffortDisarmed,
}

impl ClearPolicy {
    const fn mutates(self) -> bool {
        matches!(self, Self::OwnershipSafe | Self::BestEffortArmed)
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnershipSafe => "ownership_safe",
            Self::NonMutating => "non_mutating",
            Self::BestEffortArmed => "best_effort_armed",
            Self::BestEffortDisarmed => "best_effort_disarmed",
        }
    }
}

// Platform-union variants are intentionally dormant on any one native target.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlatformMode {
    OwnershipSafe,
    MacOs,
    Unsupported,
}

// Adapter outcomes are shared across platform-gated native implementations.
#[allow(dead_code)]
#[derive(Debug)]
enum CopyOutcome<T> {
    PreCommitFailure,
    DestructiveFailure,
    Committed(T),
    CommitUncertain(Option<T>),
    Unsupported,
}

// Adapter outcomes are shared across platform-gated native implementations.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClearOutcome {
    Cleared,
    OwnershipLost,
    Busy,
    Unsupported,
    Fatal,
}

impl From<ClearOutcome> for ReleaseOutcome {
    fn from(outcome: ClearOutcome) -> Self {
        match outcome {
            ClearOutcome::Cleared => Self::Cleared,
            ClearOutcome::OwnershipLost => Self::OwnershipLost,
            ClearOutcome::Busy => Self::Busy,
            ClearOutcome::Unsupported => Self::Unsupported,
            ClearOutcome::Fatal => Self::Fatal,
        }
    }
}

trait ClipboardAdapter: Send + 'static {
    type Token: Send + 'static;

    fn copy(&mut self, text: &str) -> CopyOutcome<Self::Token>;
    fn clear_owned(&mut self, token: &Self::Token) -> ClearOutcome;

    fn poll_interval(&self) -> Option<Duration> {
        None
    }

    fn pump(&mut self) {}
}

struct ClipboardLease<T> {
    generation: u64,
    token: T,
    copied_at: Instant,
    deadline: Option<Instant>,
    clear_policy: ClearPolicy,
    retry_at: Option<Instant>,
}

struct Quarantine<T> {
    token: T,
    retry_at: Instant,
    attempts_remaining: u8,
}

struct ActorCore<A: ClipboardAdapter> {
    adapter: A,
    mode: PlatformMode,
    timeout: Duration,
    macos_best_effort_clear: bool,
    minimum_generation: u64,
    lease: Option<ClipboardLease<A::Token>>,
    quarantine: Option<Quarantine<A::Token>>,
    last_release: Option<ReleaseOutcome>,
    release_sequence: u64,
}

impl<A: ClipboardAdapter> ActorCore<A> {
    fn new(adapter: A, mode: PlatformMode) -> Self {
        Self {
            adapter,
            mode,
            timeout: Duration::from_secs(u64::from(DEFAULT_TIMEOUT_SECONDS)),
            macos_best_effort_clear: false,
            minimum_generation: 0,
            lease: None,
            quarantine: None,
            last_release: None,
            release_sequence: 0,
        }
    }

    fn copy(
        &mut self,
        text: &str,
        generation: u64,
        now: Instant,
    ) -> Result<ClipboardStatus, ClipboardError> {
        if generation < self.minimum_generation {
            return Err(ClipboardError::StaleGeneration);
        }

        match self.adapter.copy(text) {
            CopyOutcome::PreCommitFailure => Err(ClipboardError::PreCommitFailure),
            CopyOutcome::Unsupported => Err(ClipboardError::Unsupported),
            CopyOutcome::DestructiveFailure => {
                self.lease = None;
                self.quarantine = None;
                self.last_release = None;
                Err(ClipboardError::DestructiveFailure)
            }
            CopyOutcome::CommitUncertain(token) => {
                self.lease = None;
                self.quarantine = None;
                self.last_release = None;
                if let Some(token) = token {
                    self.start_quarantine(token, now);
                }
                Err(ClipboardError::CommitUncertain)
            }
            CopyOutcome::Committed(token) => {
                let clear_policy = self.policy_for_new_copy();
                let deadline = clear_policy.mutates().then_some(now + self.timeout);
                self.quarantine = None;
                self.last_release = None;
                self.lease = Some(ClipboardLease {
                    generation,
                    token,
                    copied_at: now,
                    deadline,
                    clear_policy,
                    retry_at: None,
                });
                Ok(self.status(now))
            }
        }
    }

    fn policy_for_new_copy(&self) -> ClearPolicy {
        match self.mode {
            PlatformMode::OwnershipSafe => ClearPolicy::OwnershipSafe,
            PlatformMode::MacOs if self.macos_best_effort_clear => ClearPolicy::BestEffortArmed,
            PlatformMode::MacOs => ClearPolicy::NonMutating,
            PlatformMode::Unsupported => ClearPolicy::OwnershipSafe,
        }
    }

    fn start_quarantine(&mut self, token: A::Token, now: Instant) {
        let outcome = self.adapter.clear_owned(&token);
        self.note_release(outcome.into());
        if outcome == ClearOutcome::Busy && UNCERTAIN_MAX_ATTEMPTS > 1 {
            self.quarantine = Some(Quarantine {
                token,
                retry_at: now + BUSY_RETRY,
                attempts_remaining: UNCERTAIN_MAX_ATTEMPTS - 1,
            });
        }
    }

    fn invalidate_before(
        &mut self,
        generation: u64,
        now: Instant,
    ) -> Result<ClipboardStatus, ClipboardError> {
        self.minimum_generation = self.minimum_generation.max(generation);
        let clears_current = self
            .lease
            .as_ref()
            .is_some_and(|lease| lease.generation < generation);
        if clears_current {
            self.release(now)
        } else {
            self.last_release = None;
            Ok(self.status(now))
        }
    }

    fn release(&mut self, now: Instant) -> Result<ClipboardStatus, ClipboardError> {
        let outcome = self.attempt_release(now);
        if outcome.is_none() {
            self.last_release = None;
        }
        if outcome == Some(ReleaseOutcome::Fatal) {
            Err(ClipboardError::ReleaseFailed)
        } else {
            Ok(self.status(now))
        }
    }

    fn attempt_release(&mut self, now: Instant) -> Option<ReleaseOutcome> {
        let policy = self.lease.as_ref()?.clear_policy;
        if !policy.mutates() {
            self.lease = None;
            let outcome = ReleaseOutcome::SuppressedByPolicy;
            self.note_release(outcome);
            return Some(outcome);
        }

        let outcome = {
            let (adapter, lease) = (
                &mut self.adapter,
                self.lease.as_ref().expect("lease exists"),
            );
            adapter.clear_owned(&lease.token)
        };
        let release_outcome = ReleaseOutcome::from(outcome);
        self.note_release(release_outcome);

        match outcome {
            ClearOutcome::Busy => {
                let lease = self.lease.as_mut().expect("lease exists");
                lease.deadline = None;
                lease.retry_at = Some(now + BUSY_RETRY);
            }
            ClearOutcome::Cleared
            | ClearOutcome::OwnershipLost
            | ClearOutcome::Unsupported
            | ClearOutcome::Fatal => {
                self.lease = None;
            }
        }

        Some(release_outcome)
    }

    fn set_timeout(
        &mut self,
        seconds: u32,
        now: Instant,
    ) -> Result<ClipboardStatus, ClipboardError> {
        if !(5..=60).contains(&seconds) || seconds % 5 != 0 {
            return Err(ClipboardError::InvalidTimeout);
        }

        self.timeout = Duration::from_secs(u64::from(seconds));
        if let Some(lease) = self.lease.as_mut() {
            if lease.clear_policy.mutates() && lease.retry_at.is_none() {
                lease.deadline = Some(lease.copied_at + self.timeout);
            } else if !lease.clear_policy.mutates() {
                lease.deadline = None;
            }
        }

        if self
            .lease
            .as_ref()
            .and_then(|lease| lease.deadline)
            .is_some_and(|deadline| deadline <= now)
        {
            self.attempt_release(now);
        }

        Ok(self.status(now))
    }

    fn set_macos_best_effort(
        &mut self,
        enabled: bool,
        now: Instant,
    ) -> Result<ClipboardStatus, ClipboardError> {
        if self.mode != PlatformMode::MacOs {
            return Err(ClipboardError::Unsupported);
        }

        self.macos_best_effort_clear = enabled;
        if !enabled {
            if let Some(lease) = self.lease.as_mut() {
                if lease.clear_policy == ClearPolicy::BestEffortArmed {
                    lease.clear_policy = ClearPolicy::BestEffortDisarmed;
                    lease.deadline = None;
                    lease.retry_at = None;
                }
            }
        }

        Ok(self.status(now))
    }

    fn status(&self, now: Instant) -> ClipboardStatus {
        self.status_at(now, SystemTime::now())
    }

    fn status_at(&self, now: Instant, wall_now: SystemTime) -> ClipboardStatus {
        let (state, policy, deadline) = if self.quarantine.is_some() {
            ("release_pending", "quarantine", None)
        } else if let Some(lease) = self.lease.as_ref() {
            if lease.retry_at.is_some() {
                ("release_pending", lease.clear_policy.as_str(), None)
            } else if lease.clear_policy.mutates() {
                ("countdown", lease.clear_policy.as_str(), lease.deadline)
            } else {
                ("policy_off", lease.clear_policy.as_str(), None)
            }
        } else {
            let policy = match self.mode {
                PlatformMode::MacOs if self.macos_best_effort_clear => "best_effort_future_copies",
                PlatformMode::MacOs => "non_mutating",
                PlatformMode::OwnershipSafe => "ownership_safe",
                PlatformMode::Unsupported => "unsupported",
            };
            ("idle", policy, None)
        };

        ClipboardStatus {
            state,
            policy,
            deadline_ms: deadline.map(|deadline| deadline_millis(deadline, now, wall_now)),
            timeout_seconds: self.timeout.as_secs() as u32,
            macos_best_effort_clear: self.macos_best_effort_clear,
            last_release: self.last_release,
            release_sequence: self.release_sequence,
        }
    }

    fn next_due(&self) -> Option<Instant> {
        let lease_due = self
            .lease
            .as_ref()
            .and_then(|lease| lease.retry_at.or(lease.deadline));
        let quarantine_due = self.quarantine.as_ref().map(|value| value.retry_at);
        match (lease_due, quarantine_due) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        }
    }

    fn tick(&mut self, now: Instant) {
        if self
            .quarantine
            .as_ref()
            .is_some_and(|value| value.retry_at <= now)
        {
            self.retry_quarantine(now);
        }

        if self
            .lease
            .as_ref()
            .and_then(|lease| lease.retry_at.or(lease.deadline))
            .is_some_and(|deadline| deadline <= now)
        {
            self.attempt_release(now);
        }
    }

    fn retry_quarantine(&mut self, now: Instant) {
        let outcome = {
            let (adapter, quarantine) = (
                &mut self.adapter,
                self.quarantine.as_ref().expect("quarantine exists"),
            );
            adapter.clear_owned(&quarantine.token)
        };
        self.note_release(outcome.into());

        if outcome == ClearOutcome::Busy {
            let quarantine = self.quarantine.as_mut().expect("quarantine exists");
            if quarantine.attempts_remaining > 1 {
                quarantine.attempts_remaining -= 1;
                quarantine.retry_at = now + BUSY_RETRY;
                return;
            }
        }
        self.quarantine = None;
    }

    fn note_release(&mut self, outcome: ReleaseOutcome) {
        self.release_sequence = self.release_sequence.saturating_add(1);
        self.last_release = Some(outcome);
    }
}

fn deadline_millis(deadline: Instant, now: Instant, wall_now: SystemTime) -> u64 {
    let remaining = deadline.saturating_duration_since(now);
    let Some(wall_deadline) = wall_now.checked_add(remaining) else {
        return u64::MAX;
    };
    let since_epoch = wall_deadline.duration_since(UNIX_EPOCH).unwrap_or_default();
    u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX)
}

enum Message {
    Copy {
        secret: Zeroizing<String>,
        generation: u64,
        reply: Sender<Result<ClipboardStatus, ClipboardError>>,
    },
    Release {
        invalidated_before: u64,
        reply: Option<Sender<Result<ClipboardStatus, ClipboardError>>>,
    },
    Status {
        reply: Sender<Result<ClipboardStatus, ClipboardError>>,
    },
    SetTimeout {
        seconds: u32,
        reply: Sender<Result<ClipboardStatus, ClipboardError>>,
    },
    SetMacosBestEffort {
        enabled: bool,
        reply: Sender<Result<ClipboardStatus, ClipboardError>>,
    },
}

#[derive(Clone)]
pub struct ClipboardActor {
    sender: Sender<Message>,
}

impl ClipboardActor {
    pub fn new(app: tauri::AppHandle<tauri::Wry>) -> Self {
        #[cfg(target_os = "windows")]
        {
            drop(app);
            Self::spawn(
                windows_adapter::WindowsAdapter::new(),
                PlatformMode::OwnershipSafe,
            )
        }
        #[cfg(target_os = "macos")]
        {
            Self::spawn(macos_adapter::MacOsAdapter::new(app), PlatformMode::MacOs)
        }
        #[cfg(target_os = "linux")]
        {
            drop(app);
            let (adapter, mode) = linux_adapter::LinuxAdapter::new();
            Self::spawn(adapter, mode)
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            drop(app);
            Self::spawn(UnsupportedAdapter, PlatformMode::Unsupported)
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self::spawn(UnsupportedAdapter, PlatformMode::Unsupported)
    }

    fn spawn<A: ClipboardAdapter>(adapter: A, mode: PlatformMode) -> Self {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("localpass-clipboard".to_owned())
            .spawn(move || run_actor(ActorCore::new(adapter, mode), receiver))
            .expect("failed to start clipboard actor");
        Self { sender }
    }

    pub fn copy(
        &self,
        secret: Zeroizing<String>,
        generation: u64,
    ) -> Result<ClipboardStatus, ClipboardError> {
        let (reply, response) = mpsc::channel();
        self.sender
            .send(Message::Copy {
                secret,
                generation,
                reply,
            })
            .map_err(|_| ClipboardError::ActorUnavailable)?;
        response
            .recv()
            .map_err(|_| ClipboardError::ActorUnavailable)?
    }

    pub fn release_async(&self, invalidated_before: u64) -> Result<(), ClipboardError> {
        self.sender
            .send(Message::Release {
                invalidated_before,
                reply: None,
            })
            .map_err(|_| ClipboardError::ActorUnavailable)
    }

    pub fn release(&self, invalidated_before: u64) -> Result<ClipboardStatus, ClipboardError> {
        let (reply, response) = mpsc::channel();
        self.sender
            .send(Message::Release {
                invalidated_before,
                reply: Some(reply),
            })
            .map_err(|_| ClipboardError::ActorUnavailable)?;
        response
            .recv()
            .map_err(|_| ClipboardError::ActorUnavailable)?
    }

    pub fn status(&self) -> Result<ClipboardStatus, ClipboardError> {
        let (reply, response) = mpsc::channel();
        self.sender
            .send(Message::Status { reply })
            .map_err(|_| ClipboardError::ActorUnavailable)?;
        response
            .recv()
            .map_err(|_| ClipboardError::ActorUnavailable)?
    }

    pub fn set_timeout(&self, seconds: u32) -> Result<ClipboardStatus, ClipboardError> {
        let (reply, response) = mpsc::channel();
        self.sender
            .send(Message::SetTimeout { seconds, reply })
            .map_err(|_| ClipboardError::ActorUnavailable)?;
        response
            .recv()
            .map_err(|_| ClipboardError::ActorUnavailable)?
    }

    pub fn set_macos_best_effort(&self, enabled: bool) -> Result<ClipboardStatus, ClipboardError> {
        let (reply, response) = mpsc::channel();
        self.sender
            .send(Message::SetMacosBestEffort { enabled, reply })
            .map_err(|_| ClipboardError::ActorUnavailable)?;
        response
            .recv()
            .map_err(|_| ClipboardError::ActorUnavailable)?
    }
}

fn run_actor<A: ClipboardAdapter>(mut core: ActorCore<A>, receiver: Receiver<Message>) {
    loop {
        core.adapter.pump();
        let adapter_due = core
            .adapter
            .poll_interval()
            .map(|interval| Instant::now() + interval);
        let next_due = match (core.next_due(), adapter_due) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        };
        let message = if let Some(deadline) = next_due {
            let wait = deadline.saturating_duration_since(Instant::now());
            match receiver.recv_timeout(wait) {
                Ok(message) => Some(message),
                Err(RecvTimeoutError::Timeout) => {
                    core.tick(Instant::now());
                    None
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match receiver.recv() {
                Ok(message) => Some(message),
                Err(_) => break,
            }
        };

        let Some(message) = message else {
            continue;
        };
        let now = Instant::now();
        match message {
            Message::Copy {
                secret,
                generation,
                reply,
            } => {
                let _ = reply.send(core.copy(secret.as_str(), generation, now));
            }
            Message::Release {
                invalidated_before,
                reply,
            } => {
                let result = core.invalidate_before(invalidated_before, now);
                if let Some(reply) = reply {
                    let _ = reply.send(result);
                }
            }
            Message::Status { reply } => {
                let _ = reply.send(Ok(core.status(now)));
            }
            Message::SetTimeout { seconds, reply } => {
                let _ = reply.send(core.set_timeout(seconds, now));
            }
            Message::SetMacosBestEffort { enabled, reply } => {
                let _ = reply.send(core.set_macos_best_effort(enabled, now));
            }
        }
    }

    let _ = core.release(Instant::now());
}

// Constructed only on targets without a native adapter (and by the test helper).
#[allow(dead_code)]
struct UnsupportedAdapter;

impl ClipboardAdapter for UnsupportedAdapter {
    type Token = ();

    fn copy(&mut self, _text: &str) -> CopyOutcome<Self::Token> {
        CopyOutcome::Unsupported
    }

    fn clear_owned(&mut self, _token: &Self::Token) -> ClearOutcome {
        ClearOutcome::Unsupported
    }
}

#[cfg(target_os = "windows")]
mod windows_adapter {
    use std::{ptr, thread, time::Duration};

    use windows_sys::Win32::{
        Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardOwner, GetClipboardSequenceNumber,
                IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
                SetClipboardData,
            },
            Memory::{
                GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
            },
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, HWND_MESSAGE, MSG, PM_REMOVE,
            PeekMessageW, TranslateMessage,
        },
    };
    use zeroize::Zeroizing;

    use super::{ClearOutcome, ClipboardAdapter, CopyOutcome};

    const CF_UNICODETEXT: u32 = 13;
    const OPEN_ATTEMPTS: usize = 8;
    const OPEN_RETRY: Duration = Duration::from_millis(15);
    const EXCLUSION_FORMAT: &str = "ExcludeClipboardContentFromMonitorProcessing";
    const STATIC_WINDOW_CLASS: [u16; 7] = [83, 84, 65, 84, 73, 67, 0];
    const EMPTY_WINDOW_NAME: [u16; 1] = [0];

    pub(super) struct WindowsAdapter {
        owner_window: usize,
    }

    #[derive(Debug)]
    pub(super) struct WindowsToken {
        owner: usize,
        sequence: u32,
        marker_format: u32,
    }

    impl WindowsAdapter {
        pub(super) const fn new() -> Self {
            Self { owner_window: 0 }
        }

        fn ensure_owner(&mut self) -> Option<usize> {
            if self.owner_window != 0 {
                return Some(self.owner_window);
            }

            // Called lazily by copy on the clipboard actor thread. Drop runs
            // on that same thread, satisfying HWND create/destroy affinity.
            let owner = unsafe {
                CreateWindowExW(
                    0,
                    STATIC_WINDOW_CLASS.as_ptr(),
                    EMPTY_WINDOW_NAME.as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null(),
                )
            };
            if owner.is_null() {
                return None;
            }
            self.owner_window = owner as usize;
            Some(self.owner_window)
        }
    }

    impl ClipboardAdapter for WindowsAdapter {
        type Token = WindowsToken;

        fn copy(&mut self, text: &str) -> CopyOutcome<Self::Token> {
            if text.is_empty() {
                return CopyOutcome::PreCommitFailure;
            }
            let Some(owner) = self.ensure_owner() else {
                return CopyOutcome::PreCommitFailure;
            };

            let marker_format = register_exclusion_format();
            if marker_format == 0 {
                return CopyOutcome::PreCommitFailure;
            }

            let Some(mut marker_memory) = OwnedGlobal::allocate(4) else {
                return CopyOutcome::PreCommitFailure;
            };
            let Some(mut text_memory) = allocate_unicode_text(text) else {
                return CopyOutcome::PreCommitFailure;
            };
            let Some(clipboard) = ClipboardGuard::open(owner, OPEN_ATTEMPTS) else {
                return CopyOutcome::PreCommitFailure;
            };

            if unsafe { EmptyClipboard() } == 0 {
                return CopyOutcome::PreCommitFailure;
            }

            if unsafe { SetClipboardData(marker_format, marker_memory.raw() as HANDLE) }.is_null() {
                return CopyOutcome::DestructiveFailure;
            }
            marker_memory.release_to_system();

            if unsafe { SetClipboardData(CF_UNICODETEXT, text_memory.raw() as HANDLE) }.is_null() {
                unsafe {
                    EmptyClipboard();
                }
                return CopyOutcome::DestructiveFailure;
            }
            text_memory.release_to_system();
            drop(clipboard);

            let token = WindowsToken {
                owner,
                sequence: unsafe { GetClipboardSequenceNumber() },
                marker_format,
            };
            if matches_owned(&token) {
                CopyOutcome::Committed(token)
            } else {
                CopyOutcome::CommitUncertain(Some(token))
            }
        }

        fn clear_owned(&mut self, token: &Self::Token) -> ClearOutcome {
            if !matches_owned(token) {
                return ClearOutcome::OwnershipLost;
            }

            let Some(_clipboard) = ClipboardGuard::open(token.owner, OPEN_ATTEMPTS) else {
                return ClearOutcome::Busy;
            };
            if !matches_owned(token) {
                return ClearOutcome::OwnershipLost;
            }

            if unsafe { EmptyClipboard() } != 0 {
                ClearOutcome::Cleared
            } else {
                ClearOutcome::Busy
            }
        }

        fn poll_interval(&self) -> Option<Duration> {
            (self.owner_window != 0).then_some(OPEN_RETRY)
        }

        fn pump(&mut self) {
            if self.owner_window == 0 {
                return;
            }

            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, self.owner_window as HWND, 0, 0, PM_REMOVE) }
                != 0
            {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }

    impl Drop for WindowsAdapter {
        fn drop(&mut self) {
            if self.owner_window == 0 {
                return;
            }

            unsafe {
                DestroyWindow(self.owner_window as HWND);
            }
            self.owner_window = 0;
        }
    }

    fn register_exclusion_format() -> u32 {
        let name: Vec<u16> = EXCLUSION_FORMAT.encode_utf16().chain(Some(0)).collect();
        unsafe { RegisterClipboardFormatW(name.as_ptr()) }
    }

    fn matches_owned(token: &WindowsToken) -> bool {
        token.sequence != 0
            && token.marker_format != 0
            && unsafe { GetClipboardSequenceNumber() } == token.sequence
            && unsafe { GetClipboardOwner() } == token.owner as HWND
            && unsafe { IsClipboardFormatAvailable(token.marker_format) } != 0
    }

    fn allocate_unicode_text(text: &str) -> Option<OwnedGlobal> {
        let wide = Zeroizing::new(text.encode_utf16().chain(Some(0)).collect::<Vec<_>>());
        let byte_count = wide.len().checked_mul(size_of::<u16>())?;
        let memory = OwnedGlobal::allocate(byte_count)?;
        let target = unsafe { GlobalLock(memory.raw()) }.cast::<u16>();
        if target.is_null() {
            return None;
        }

        unsafe {
            ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
            GlobalUnlock(memory.raw());
        }
        Some(memory)
    }

    struct ClipboardGuard;

    impl ClipboardGuard {
        fn open(owner: usize, attempts: usize) -> Option<Self> {
            for attempt in 0..attempts {
                if unsafe { OpenClipboard(owner as HWND) } != 0 {
                    return Some(Self);
                }
                if attempt + 1 < attempts {
                    thread::sleep(OPEN_RETRY);
                }
            }
            None
        }
    }

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }

    struct OwnedGlobal {
        handle: HGLOBAL,
    }

    impl OwnedGlobal {
        fn allocate(bytes: usize) -> Option<Self> {
            let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes) };
            (!handle.is_null()).then_some(Self { handle })
        }

        fn raw(&self) -> HGLOBAL {
            self.handle
        }

        fn release_to_system(&mut self) {
            self.handle = ptr::null_mut();
        }
    }

    impl Drop for OwnedGlobal {
        fn drop(&mut self) {
            if self.handle.is_null() {
                return;
            }

            unsafe {
                let size = GlobalSize(self.handle);
                let target = GlobalLock(self.handle).cast::<u8>();
                if !target.is_null() {
                    ptr::write_bytes(target, 0, size);
                    GlobalUnlock(self.handle);
                }
                GlobalFree(self.handle);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;

        use super::*;

        #[test]
        fn owner_window_is_lazy_stable_and_destroyed_on_actor_drop() {
            let mut adapter = WindowsAdapter::new();
            assert_eq!(adapter.owner_window, 0);
            assert_eq!(adapter.poll_interval(), None);

            let owner = adapter.ensure_owner().expect("message-only HWND");
            assert_ne!(owner, 0);
            assert_ne!(unsafe { IsWindow(owner as HWND) }, 0);
            assert_eq!(adapter.ensure_owner(), Some(owner));
            assert_eq!(adapter.poll_interval(), Some(OPEN_RETRY));
            adapter.pump();

            drop(adapter);
            assert_eq!(unsafe { IsWindow(owner as HWND) }, 0);
        }
    }
}

#[cfg(target_os = "linux")]
mod linux_adapter {
    use std::{
        env,
        sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender},
        thread,
        time::Duration,
    };

    use x11rb::{
        COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE,
        connection::Connection,
        protocol::{
            Event,
            xproto::{
                Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, PropMode,
                SELECTION_NOTIFY_EVENT, SelectionClearEvent, SelectionNotifyEvent,
                SelectionRequestEvent, Window, WindowClass,
            },
        },
        rust_connection::RustConnection,
        wrapper::ConnectionExt as _,
    };
    use zeroize::Zeroizing;

    use super::{
        ClearOutcome, ClipboardAdapter, CopyOutcome, LinuxSession, PlatformMode, X11LeaseToken,
        X11TargetAtoms, X11TargetReply, classify_linux_session, x11_display_is_local,
        x11_token_matches,
    };

    const EVENT_POLL: Duration = Duration::from_millis(5);
    const KDE_SECRET_HINT: &[u8] = b"secret";

    pub(super) enum LinuxAdapter {
        Unsupported,
        Running(Sender<ServerCommand>),
    }

    enum ServerCommand {
        Copy {
            payload: Zeroizing<String>,
            reply: SyncSender<CopyOutcome<X11LeaseToken>>,
        },
        Clear {
            token: X11LeaseToken,
            reply: SyncSender<ClearOutcome>,
        },
    }

    struct X11Lease {
        token: X11LeaseToken,
        payload: Zeroizing<String>,
    }

    struct X11Server {
        connection: RustConnection,
        root: Window,
        root_visual: u32,
        clipboard: Atom,
        atoms: X11TargetAtoms,
        active: Option<X11Lease>,
        next_lease_id: u64,
    }

    impl LinuxAdapter {
        pub(super) fn new() -> (Self, PlatformMode) {
            let wayland_display_present = env::var_os("WAYLAND_DISPLAY").is_some();
            let raw_session_type = env::var_os("XDG_SESSION_TYPE");
            let session_type = raw_session_type.as_deref().and_then(|value| value.to_str());
            let invalid_session_type = raw_session_type
                .as_deref()
                .is_some_and(|value| value.to_str().is_none());
            let raw_display = env::var_os("DISPLAY");
            let display = raw_display.as_deref().and_then(|value| value.to_str());
            let local_display_present = x11_display_is_local(display);
            let session = if invalid_session_type {
                LinuxSession::Unavailable
            } else {
                classify_linux_session(wayland_display_present, session_type, local_display_present)
            };

            match session {
                LinuxSession::X11 => match start_server() {
                    Ok(sender) => (Self::Running(sender), PlatformMode::OwnershipSafe),
                    Err(()) => (Self::Unsupported, PlatformMode::Unsupported),
                },
                LinuxSession::Wayland | LinuxSession::Unavailable => {
                    (Self::Unsupported, PlatformMode::Unsupported)
                }
            }
        }
    }

    impl ClipboardAdapter for LinuxAdapter {
        type Token = X11LeaseToken;

        fn copy(&mut self, text: &str) -> CopyOutcome<Self::Token> {
            if text.is_empty() {
                return CopyOutcome::PreCommitFailure;
            }

            match self {
                Self::Unsupported => CopyOutcome::Unsupported,
                Self::Running(sender) => {
                    let (reply, response) = mpsc::sync_channel(1);
                    let command = ServerCommand::Copy {
                        payload: Zeroizing::new(text.to_owned()),
                        reply,
                    };
                    if sender.send(command).is_err() {
                        return CopyOutcome::DestructiveFailure;
                    }
                    response.recv().unwrap_or(CopyOutcome::DestructiveFailure)
                }
            }
        }

        fn clear_owned(&mut self, token: &Self::Token) -> ClearOutcome {
            match self {
                Self::Unsupported => ClearOutcome::Unsupported,
                Self::Running(sender) => {
                    let (reply, response) = mpsc::sync_channel(1);
                    if sender
                        .send(ServerCommand::Clear {
                            token: *token,
                            reply,
                        })
                        .is_err()
                    {
                        return ClearOutcome::OwnershipLost;
                    }
                    response.recv().unwrap_or(ClearOutcome::OwnershipLost)
                }
            }
        }
    }

    fn start_server() -> Result<Sender<ServerCommand>, ()> {
        let (sender, receiver) = mpsc::channel();
        let (ready, started) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("localpass-x11-selection".to_owned())
            .spawn(move || match X11Server::connect() {
                Ok(mut server) => {
                    if ready.send(Ok(())).is_ok() {
                        server.run(receiver);
                    }
                }
                Err(()) => {
                    let _ = ready.send(Err(()));
                }
            })
            .map_err(|_| ())?;

        started.recv().map_err(|_| ())??;
        Ok(sender)
    }

    impl X11Server {
        fn connect() -> Result<Self, ()> {
            let (connection, screen_number) = RustConnection::connect(None).map_err(|_| ())?;
            let screen = connection.setup().roots.get(screen_number).ok_or(())?;
            let root = screen.root;
            let root_visual = screen.root_visual;
            let clipboard = intern(&connection, b"CLIPBOARD")?;
            let atoms = X11TargetAtoms {
                targets: intern(&connection, b"TARGETS")?,
                utf8_string: intern(&connection, b"UTF8_STRING")?,
                text: intern(&connection, b"TEXT")?,
                string: AtomEnum::STRING.into(),
                text_plain_utf8: intern(&connection, b"text/plain;charset=utf-8")?,
                text_plain: intern(&connection, b"text/plain")?,
                kde_hint: intern(&connection, b"x-kde-passwordManagerHint")?,
                save_targets: intern(&connection, b"SAVE_TARGETS")?,
            };

            Ok(Self {
                connection,
                root,
                root_visual,
                clipboard,
                atoms,
                active: None,
                next_lease_id: 1,
            })
        }

        fn run(&mut self, receiver: Receiver<ServerCommand>) {
            loop {
                loop {
                    match self.connection.poll_for_event() {
                        Ok(Some(event)) => self.handle_event(event),
                        Ok(None) => break,
                        Err(_) => {
                            self.active = None;
                            return;
                        }
                    }
                }

                match receiver.recv_timeout(EVENT_POLL) {
                    Ok(ServerCommand::Copy { payload, reply }) => {
                        let _ = reply.send(self.copy(payload));
                    }
                    Ok(ServerCommand::Clear { token, reply }) => {
                        let _ = reply.send(self.clear(token));
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }

            self.destroy_active_best_effort();
        }

        fn copy(&mut self, payload: Zeroizing<String>) -> CopyOutcome<X11LeaseToken> {
            let window = match self.connection.generate_id() {
                Ok(window) => window,
                Err(_) => return CopyOutcome::PreCommitFailure,
            };
            let create = match self.connection.create_window(
                COPY_DEPTH_FROM_PARENT,
                window,
                self.root,
                0,
                0,
                1,
                1,
                0,
                WindowClass::INPUT_OUTPUT,
                self.root_visual,
                &CreateWindowAux::new(),
            ) {
                Ok(cookie) => cookie,
                Err(_) => return CopyOutcome::PreCommitFailure,
            };
            if create.check().is_err() {
                self.destroy_window_best_effort(window);
                return CopyOutcome::PreCommitFailure;
            }

            let token = X11LeaseToken {
                lease_id: self.next_lease_id,
                window,
            };
            self.next_lease_id = self.next_lease_id.wrapping_add(1);
            let set_owner =
                match self
                    .connection
                    .set_selection_owner(window, self.clipboard, CURRENT_TIME)
                {
                    Ok(cookie) => cookie,
                    Err(_) => {
                        self.destroy_window_best_effort(window);
                        return CopyOutcome::PreCommitFailure;
                    }
                };

            if set_owner.check().is_err() {
                if self.connection_alive() {
                    self.destroy_window_best_effort(window);
                    return CopyOutcome::PreCommitFailure;
                }
                return self.install_uncertain(token, payload);
            }

            let owner = match self.selection_owner() {
                Some(owner) => owner,
                None => return self.install_uncertain(token, payload),
            };
            if owner != window {
                self.destroy_window_best_effort(window);
                self.destroy_active_best_effort();
                return CopyOutcome::DestructiveFailure;
            }

            let previous = self.active.replace(X11Lease { token, payload });
            if let Some(previous) = previous {
                if !self.destroy_window_checked(previous.token.window) {
                    return CopyOutcome::CommitUncertain(Some(token));
                }
            }

            CopyOutcome::Committed(token)
        }

        fn install_uncertain(
            &mut self,
            token: X11LeaseToken,
            payload: Zeroizing<String>,
        ) -> CopyOutcome<X11LeaseToken> {
            let previous = self.active.replace(X11Lease { token, payload });
            if let Some(previous) = previous {
                self.destroy_window_best_effort(previous.token.window);
            }
            CopyOutcome::CommitUncertain(Some(token))
        }

        fn clear(&mut self, requested: X11LeaseToken) -> ClearOutcome {
            if !x11_token_matches(self.active.as_ref().map(|lease| lease.token), requested) {
                return ClearOutcome::OwnershipLost;
            }

            let Some(owner) = self.selection_owner() else {
                self.destroy_active_best_effort();
                return ClearOutcome::Fatal;
            };
            if owner != requested.window {
                self.destroy_active_best_effort();
                return ClearOutcome::OwnershipLost;
            }

            let active = self.active.take().expect("matching X11 lease disappeared");
            if self.destroy_window_checked(active.token.window) {
                ClearOutcome::Cleared
            } else if self.connection_alive() {
                ClearOutcome::OwnershipLost
            } else {
                ClearOutcome::Fatal
            }
        }

        fn handle_event(&mut self, event: Event) {
            match event {
                Event::SelectionRequest(request) => self.handle_selection_request(request),
                Event::SelectionClear(clear) => self.handle_selection_clear(clear),
                _ => {}
            }
        }

        fn handle_selection_clear(&mut self, clear: SelectionClearEvent) {
            let matches_active = clear.selection == self.clipboard
                && self
                    .active
                    .as_ref()
                    .is_some_and(|lease| lease.token.window == clear.owner);
            if matches_active {
                self.destroy_active_best_effort();
            }
        }

        fn handle_selection_request(&mut self, request: SelectionRequestEvent) {
            let property = if request.property == NONE {
                request.target
            } else {
                request.property
            };
            let valid_owner = request.selection == self.clipboard
                && self
                    .active
                    .as_ref()
                    .is_some_and(|lease| lease.token.window == request.owner);
            let written = valid_owner && self.write_requested_property(&request, property);
            let notify = SelectionNotifyEvent {
                response_type: SELECTION_NOTIFY_EVENT,
                sequence: 0,
                time: request.time,
                requestor: request.requestor,
                selection: request.selection,
                target: request.target,
                property: if written { property } else { NONE },
            };

            if let Ok(cookie) =
                self.connection
                    .send_event(false, request.requestor, EventMask::NO_EVENT, notify)
            {
                let _ = cookie.check();
            }
            let _ = self.connection.flush();
        }

        fn write_requested_property(
            &self,
            request: &SelectionRequestEvent,
            property: Atom,
        ) -> bool {
            let Some(active) = self.active.as_ref() else {
                return false;
            };
            let result = match self.atoms.reply_for(request.target) {
                X11TargetReply::Targets => {
                    let offered = self.atoms.offered();
                    self.connection.change_property32(
                        PropMode::REPLACE,
                        request.requestor,
                        property,
                        AtomEnum::ATOM,
                        &offered,
                    )
                }
                X11TargetReply::Text(data_type) => self.connection.change_property8(
                    PropMode::REPLACE,
                    request.requestor,
                    property,
                    data_type,
                    active.payload.as_bytes(),
                ),
                X11TargetReply::Hint => self.connection.change_property8(
                    PropMode::REPLACE,
                    request.requestor,
                    property,
                    self.atoms.kde_hint,
                    KDE_SECRET_HINT,
                ),
                X11TargetReply::Reject => return false,
            };

            result.is_ok_and(|cookie| cookie.check().is_ok())
        }

        fn selection_owner(&self) -> Option<Window> {
            self.connection
                .get_selection_owner(self.clipboard)
                .ok()?
                .reply()
                .ok()
                .map(|reply| reply.owner)
        }

        fn connection_alive(&self) -> bool {
            self.connection
                .get_input_focus()
                .is_ok_and(|cookie| cookie.reply().is_ok())
        }

        fn destroy_window_checked(&self, window: Window) -> bool {
            self.connection
                .destroy_window(window)
                .is_ok_and(|cookie| cookie.check().is_ok())
        }

        fn destroy_window_best_effort(&self, window: Window) {
            let _ = self.connection.destroy_window(window);
            let _ = self.connection.flush();
        }

        fn destroy_active_best_effort(&mut self) {
            if let Some(active) = self.active.take() {
                self.destroy_window_best_effort(active.token.window);
            }
        }
    }

    fn intern(connection: &RustConnection, name: &[u8]) -> Result<Atom, ()> {
        connection
            .intern_atom(false, name)
            .map_err(|_| ())?
            .reply()
            .map(|reply| reply.atom)
            .map_err(|_| ())
    }
}

#[cfg(target_os = "macos")]
mod macos_adapter {
    use objc2_app_kit::{NSPasteboard, NSPasteboardContentsOptions, NSPasteboardTypeString};
    use objc2_foundation::{NSData, NSString};
    use tauri::{AppHandle, Wry};
    use zeroize::Zeroizing;

    use super::{BridgeError, ClearOutcome, ClipboardAdapter, CopyOutcome, run_scheduled};

    const COOPERATIVE_TYPES: [&str; 3] = [
        "org.nspasteboard.ConcealedType",
        "org.nspasteboard.TransientType",
        "org.nspasteboard.AutoGeneratedType",
    ];

    pub(super) struct MacOsAdapter {
        app: AppHandle<Wry>,
    }

    #[derive(Clone, Copy, Debug)]
    pub(super) struct MacOsToken {
        change_count: isize,
    }

    impl MacOsAdapter {
        pub(super) fn new(app: AppHandle<Wry>) -> Self {
            Self { app }
        }
    }

    impl ClipboardAdapter for MacOsAdapter {
        type Token = MacOsToken;

        fn copy(&mut self, text: &str) -> CopyOutcome<Self::Token> {
            if text.is_empty() {
                return CopyOutcome::PreCommitFailure;
            }

            let secret = Zeroizing::new(text.to_owned());
            match run_scheduled(
                |task| self.app.run_on_main_thread(task),
                move || copy_on_main(secret),
            ) {
                Ok(outcome) => outcome,
                Err(BridgeError::ScheduleFailed) => CopyOutcome::PreCommitFailure,
                Err(BridgeError::CompletionLost) => CopyOutcome::CommitUncertain(None),
            }
        }

        fn clear_owned(&mut self, token: &Self::Token) -> ClearOutcome {
            let expected = token.change_count;
            match run_scheduled(
                |task| self.app.run_on_main_thread(task),
                move || clear_on_main(expected),
            ) {
                Ok(outcome) => outcome,
                Err(_) => ClearOutcome::Fatal,
            }
        }
    }

    fn copy_on_main(secret: Zeroizing<String>) -> CopyOutcome<MacOsToken> {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.prepareForNewContentsWithOptions(NSPasteboardContentsOptions::CurrentHostOnly);

        let empty = NSData::with_bytes(&[]);
        for type_name in COOPERATIVE_TYPES {
            let data_type = NSString::from_str(type_name);
            if !pasteboard.setData_forType(Some(&empty), &data_type) {
                // Do not clear again: another writer may have won after prepare
                // replaced the prior contents.
                return CopyOutcome::DestructiveFailure;
            }
        }

        let text = NSString::from_str(secret.as_str());
        // SAFETY: AppKit exports this immutable process-lifetime pasteboard type.
        let string_type = unsafe { NSPasteboardTypeString };
        if !pasteboard.setString_forType(&text, string_type) {
            // Marker-only content is safer than racing a newer clipboard owner;
            // the failed call did not publish the secret.
            return CopyOutcome::DestructiveFailure;
        }

        CopyOutcome::Committed(MacOsToken {
            change_count: pasteboard.changeCount(),
        })
    }

    fn clear_on_main(expected: isize) -> ClearOutcome {
        let pasteboard = NSPasteboard::generalPasteboard();
        if pasteboard.changeCount() != expected {
            return ClearOutcome::OwnershipLost;
        }

        // NSPasteboard has no compare-and-clear primitive. Another writer can
        // still land between this ownership check and clearContents.
        pasteboard.clearContents();
        ClearOutcome::Cleared
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    use tauri::ipc::{InvokeResponseBody, IpcResponse};
    use zeroize::Zeroizing;

    use super::*;

    struct MockAdapter {
        copy_outcomes: VecDeque<CopyOutcome<u64>>,
        clear_outcomes: VecDeque<ClearOutcome>,
        clear_calls: Vec<u64>,
    }

    impl MockAdapter {
        fn new(
            copy_outcomes: impl IntoIterator<Item = CopyOutcome<u64>>,
            clear_outcomes: impl IntoIterator<Item = ClearOutcome>,
        ) -> Self {
            Self {
                copy_outcomes: copy_outcomes.into_iter().collect(),
                clear_outcomes: clear_outcomes.into_iter().collect(),
                clear_calls: Vec::new(),
            }
        }
    }

    impl ClipboardAdapter for MockAdapter {
        type Token = u64;

        fn copy(&mut self, _text: &str) -> CopyOutcome<Self::Token> {
            self.copy_outcomes
                .pop_front()
                .expect("missing mock copy outcome")
        }

        fn clear_owned(&mut self, token: &Self::Token) -> ClearOutcome {
            self.clear_calls.push(*token);
            self.clear_outcomes
                .pop_front()
                .unwrap_or(ClearOutcome::OwnershipLost)
        }
    }

    #[test]
    fn copy_phase_outcomes_preserve_or_invalidate_prior_lease() {
        let start = Instant::now();
        let adapter = MockAdapter::new(
            [
                CopyOutcome::Committed(1),
                CopyOutcome::PreCommitFailure,
                CopyOutcome::DestructiveFailure,
            ],
            [],
        );
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.copy("one", 1, start).unwrap();
        let first_generation = core.lease.as_ref().unwrap().generation;
        assert_eq!(
            core.copy("two", 1, start + Duration::from_secs(1)),
            Err(ClipboardError::PreCommitFailure)
        );
        assert_eq!(core.lease.as_ref().unwrap().token, 1);
        assert_eq!(core.lease.as_ref().unwrap().generation, first_generation);

        assert_eq!(
            core.copy("three", 1, start + Duration::from_secs(2)),
            Err(ClipboardError::DestructiveFailure)
        );
        assert!(core.lease.is_none());
    }

    #[test]
    fn commit_uncertain_uses_bounded_quarantine_retry() {
        let start = Instant::now();
        let adapter = MockAdapter::new(
            [CopyOutcome::CommitUncertain(Some(7))],
            std::iter::repeat_n(ClearOutcome::Busy, usize::from(UNCERTAIN_MAX_ATTEMPTS)),
        );
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        assert_eq!(
            core.copy("secret", 1, start),
            Err(ClipboardError::CommitUncertain)
        );
        assert!(core.lease.is_none());
        assert!(core.quarantine.is_some());

        for attempt in 1..UNCERTAIN_MAX_ATTEMPTS {
            core.tick(start + BUSY_RETRY * u32::from(attempt));
        }

        assert!(core.quarantine.is_none());
        assert_eq!(
            core.adapter.clear_calls.len(),
            usize::from(UNCERTAIN_MAX_ATTEMPTS)
        );
    }

    #[test]
    fn timeout_recomputes_from_original_copy_time() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(1)], [ClearOutcome::Cleared]);
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.copy("secret", 1, start).unwrap();
        let status = core.set_timeout(5, start + Duration::from_secs(6)).unwrap();

        assert_eq!(status.last_release, Some(ReleaseOutcome::Cleared));
        assert_eq!(status.release_sequence, 1);
        assert!(core.lease.is_none());
        assert_eq!(core.adapter.clear_calls, [1]);
    }

    #[test]
    fn no_op_clear_after_timeout_does_not_replay_prior_release() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(1)], [ClearOutcome::Cleared]);
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.copy("secret", 1, start).unwrap();
        core.tick(start + Duration::from_secs(30));
        let released = core.status(start + Duration::from_secs(30));
        assert_eq!(released.last_release, Some(ReleaseOutcome::Cleared));
        assert_eq!(released.release_sequence, 1);

        let status = core
            .invalidate_before(2, start + Duration::from_secs(31))
            .unwrap();
        assert_eq!(status.state, "idle");
        assert_eq!(status.last_release, None);
        assert_eq!(status.release_sequence, released.release_sequence);
        assert_eq!(core.adapter.clear_calls, [1]);
    }

    #[test]
    fn new_copy_cancels_stale_generation_deadline() {
        let start = Instant::now();
        let adapter = MockAdapter::new(
            [CopyOutcome::Committed(1), CopyOutcome::Committed(2)],
            [ClearOutcome::Cleared],
        );
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.copy("one", 1, start).unwrap();
        core.copy("two", 2, start + Duration::from_secs(1)).unwrap();
        core.tick(start + Duration::from_secs(30));
        assert!(core.adapter.clear_calls.is_empty());

        core.tick(start + Duration::from_secs(31));
        assert_eq!(core.adapter.clear_calls, [2]);
        assert!(core.lease.is_none());
    }

    #[test]
    fn busy_release_retries_at_250_milliseconds() {
        let start = Instant::now();
        let adapter = MockAdapter::new(
            [CopyOutcome::Committed(4)],
            [ClearOutcome::Busy, ClearOutcome::Cleared],
        );
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.copy("secret", 1, start).unwrap();
        let status = core.release(start).unwrap();
        assert_eq!(status.state, "release_pending");
        core.tick(start + BUSY_RETRY - Duration::from_millis(1));
        assert_eq!(core.adapter.clear_calls, [4]);

        core.tick(start + BUSY_RETRY);
        assert_eq!(core.adapter.clear_calls, [4, 4]);
        assert!(core.lease.is_none());
    }

    #[test]
    fn macos_policy_is_per_lease_and_disarm_is_irreversible() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(1), CopyOutcome::Committed(2)], []);
        let mut core = ActorCore::new(adapter, PlatformMode::MacOs);

        let off = core.copy("off", 1, start).unwrap();
        assert_eq!(off.state, "policy_off");
        assert_eq!(off.deadline_ms, None);

        core.set_macos_best_effort(true, start + Duration::from_secs(1))
            .unwrap();
        core.set_timeout(5, start + Duration::from_secs(2)).unwrap();
        assert_eq!(
            core.lease.as_ref().unwrap().clear_policy,
            ClearPolicy::NonMutating
        );
        core.release(start + Duration::from_secs(3)).unwrap();
        assert!(core.adapter.clear_calls.is_empty());

        core.copy("armed", 2, start + Duration::from_secs(4))
            .unwrap();
        assert_eq!(
            core.lease.as_ref().unwrap().clear_policy,
            ClearPolicy::BestEffortArmed
        );
        core.set_macos_best_effort(false, start + Duration::from_secs(5))
            .unwrap();
        core.set_macos_best_effort(true, start + Duration::from_secs(6))
            .unwrap();
        core.set_timeout(10, start + Duration::from_secs(7))
            .unwrap();
        assert_eq!(
            core.lease.as_ref().unwrap().clear_policy,
            ClearPolicy::BestEffortDisarmed
        );
        assert_eq!(core.lease.as_ref().unwrap().deadline, None);
        core.release(start + Duration::from_secs(8)).unwrap();
        assert!(core.adapter.clear_calls.is_empty());
    }

    #[test]
    fn macos_disable_reports_active_lease_disarmed_without_release() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(9)], []);
        let mut core = ActorCore::new(adapter, PlatformMode::MacOs);

        core.set_macos_best_effort(true, start).unwrap();
        core.copy("armed", 1, start).unwrap();
        let status = core
            .set_macos_best_effort(false, start + Duration::from_secs(1))
            .unwrap();

        assert_eq!(status.state, "policy_off");
        assert_eq!(status.policy, "best_effort_disarmed");
        assert_eq!(status.last_release, None);
        assert_eq!(status.release_sequence, 0);
        assert_eq!(
            core.lease.as_ref().unwrap().clear_policy,
            ClearPolicy::BestEffortDisarmed
        );
    }

    #[test]
    fn macos_disable_ack_reports_release_that_already_finished() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(9)], [ClearOutcome::Cleared]);
        let mut core = ActorCore::new(adapter, PlatformMode::MacOs);

        core.set_macos_best_effort(true, start).unwrap();
        core.copy("armed", 1, start).unwrap();
        let prior = core.status(start + Duration::from_secs(29));
        core.tick(start + Duration::from_secs(30));
        let status = core
            .set_macos_best_effort(false, start + Duration::from_secs(30))
            .unwrap();

        assert_eq!(status.state, "idle");
        assert_eq!(status.policy, "non_mutating");
        assert_eq!(status.last_release, Some(ReleaseOutcome::Cleared));
        assert!(status.release_sequence > prior.release_sequence);
        assert!(!status.macos_best_effort_clear);
    }

    #[test]
    fn macos_disable_sequence_identifies_an_already_observed_release() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(9)], [ClearOutcome::Cleared]);
        let mut core = ActorCore::new(adapter, PlatformMode::MacOs);

        core.set_macos_best_effort(true, start).unwrap();
        core.copy("armed", 1, start).unwrap();
        core.tick(start + Duration::from_secs(30));
        let prior = core.status(start + Duration::from_secs(31));
        let status = core
            .set_macos_best_effort(false, start + Duration::from_secs(32))
            .unwrap();

        assert_eq!(status.policy, "non_mutating");
        assert_eq!(status.last_release, Some(ReleaseOutcome::Cleared));
        assert_eq!(status.release_sequence, prior.release_sequence);
    }

    #[test]
    fn actor_fifo_does_not_rearm_disarmed_macos_lease() {
        let adapter = MockAdapter::new([CopyOutcome::Committed(11)], []);
        let actor = ClipboardActor::spawn(adapter, PlatformMode::MacOs);

        actor.set_macos_best_effort(true).unwrap();
        actor.copy(Zeroizing::new("secret".to_owned()), 1).unwrap();
        actor.set_macos_best_effort(false).unwrap();
        actor.set_macos_best_effort(true).unwrap();
        actor.set_timeout(5).unwrap();
        let status = actor.status().unwrap();

        assert_eq!(status.policy, "best_effort_disarmed");
        assert_eq!(status.deadline_ms, None);
        assert_eq!(
            actor.release(2).unwrap().last_release,
            Some(ReleaseOutcome::SuppressedByPolicy)
        );
    }

    #[test]
    fn macos_consent_starts_off_for_every_actor_core() {
        let first = ActorCore::new(MockAdapter::new([], []), PlatformMode::MacOs);
        let second = ActorCore::new(MockAdapter::new([], []), PlatformMode::MacOs);

        assert!(!first.macos_best_effort_clear);
        assert!(!second.macos_best_effort_clear);
    }

    #[test]
    fn macos_policy_off_never_calls_clear_adapter() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(3)], []);
        let mut core = ActorCore::new(adapter, PlatformMode::MacOs);

        core.copy("off", 1, start).unwrap();
        core.tick(start + Duration::from_secs(300));
        let status = core.release(start + Duration::from_secs(301)).unwrap();

        assert_eq!(
            status.last_release,
            Some(ReleaseOutcome::SuppressedByPolicy)
        );
        assert!(core.adapter.clear_calls.is_empty());
    }

    #[test]
    fn clipboard_status_and_release_outcomes_keep_ipc_wire_contract() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(1)], []);
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);
        core.copy("secret", 1, start).unwrap();

        let wall_now = UNIX_EPOCH + Duration::from_secs(1_000);
        let status = core.status_at(start, wall_now);

        assert_eq!(status.deadline_ms, Some(1_030_000));
        let InvokeResponseBody::Json(json) = status.body().unwrap() else {
            panic!("clipboard status must use JSON IPC");
        };
        assert_eq!(
            json,
            concat!(
                r#"{"state":"countdown","policy":"ownership_safe","deadline_ms":1030000,"#,
                r#""timeout_seconds":30,"macos_best_effort_clear":false,"#,
                r#""last_release":null,"release_sequence":0}"#,
            )
        );
        for (outcome, wire_name) in [
            (ReleaseOutcome::Cleared, "cleared"),
            (ReleaseOutcome::OwnershipLost, "ownership_lost"),
            (ReleaseOutcome::Busy, "busy"),
            (ReleaseOutcome::Unsupported, "unsupported"),
            (ReleaseOutcome::Fatal, "fatal"),
            (ReleaseOutcome::SuppressedByPolicy, "suppressed_by_policy"),
        ] {
            assert_eq!(
                outcome.body().unwrap().deserialize::<String>().unwrap(),
                wire_name
            );
        }
    }

    #[test]
    fn release_fence_rejects_stale_copy_before_adapter_commit() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(2)], []);
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.invalidate_before(2, start).unwrap();
        assert_eq!(
            core.copy("stale", 1, start),
            Err(ClipboardError::StaleGeneration)
        );
        assert_eq!(core.adapter.copy_outcomes.len(), 1);
        core.copy("current", 2, start).unwrap();
    }

    #[test]
    fn release_fence_after_copy_clears_only_older_generation() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(1)], [ClearOutcome::Cleared]);
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.copy("old", 1, start).unwrap();
        let status = core.invalidate_before(2, start).unwrap();

        assert_eq!(status.last_release, Some(ReleaseOutcome::Cleared));
        assert!(core.lease.is_none());
        assert_eq!(core.adapter.clear_calls, [1]);
    }

    #[test]
    fn stale_release_fence_cannot_clear_current_generation() {
        let start = Instant::now();
        let adapter = MockAdapter::new([CopyOutcome::Committed(2)], []);
        let mut core = ActorCore::new(adapter, PlatformMode::OwnershipSafe);

        core.invalidate_before(2, start).unwrap();
        core.copy("current", 2, start).unwrap();
        core.invalidate_before(2, start).unwrap();

        assert_eq!(core.lease.as_ref().unwrap().generation, 2);
        assert!(core.adapter.clear_calls.is_empty());
    }

    #[test]
    fn actor_fifo_rejects_copy_queued_after_newer_release_fence() {
        let actor = ClipboardActor::spawn(MockAdapter::new([], []), PlatformMode::OwnershipSafe);

        actor.release_async(2).unwrap();
        assert_eq!(
            actor.copy(Zeroizing::new("stale".to_owned()), 1),
            Err(ClipboardError::StaleGeneration)
        );
    }

    #[test]
    fn x11_display_guard_allows_only_local_unix_forms() {
        assert!(x11_display_is_local(Some(":0")));
        assert!(x11_display_is_local(Some(":12.1")));
        assert!(x11_display_is_local(Some("unix:0")));
        assert!(x11_display_is_local(Some("unix/:0.0")));

        assert!(!x11_display_is_local(None));
        assert!(!x11_display_is_local(Some("")));
        assert!(!x11_display_is_local(Some("localhost:10.0")));
        assert!(!x11_display_is_local(Some("127.0.0.1:10.0")));
        assert!(!x11_display_is_local(Some("host.example:0")));
        assert!(!x11_display_is_local(Some(":0.extra")));
        assert!(!x11_display_is_local(Some(":x")));
    }

    #[test]
    fn linux_session_detection_never_falls_back_from_wayland_to_xwayland() {
        assert_eq!(
            classify_linux_session(true, Some("x11"), true),
            LinuxSession::Wayland
        );
        assert_eq!(
            classify_linux_session(false, Some("wayland"), true),
            LinuxSession::Wayland
        );
        assert_eq!(
            classify_linux_session(false, Some(" WayLand "), true),
            LinuxSession::Wayland
        );
        assert_eq!(
            classify_linux_session(false, Some("x11"), true),
            LinuxSession::X11
        );
        assert_eq!(classify_linux_session(false, None, true), LinuxSession::X11);
        assert_eq!(
            classify_linux_session(false, Some(""), true),
            LinuxSession::X11
        );
        assert_eq!(
            classify_linux_session(false, Some("tty"), true),
            LinuxSession::Unavailable
        );
        assert_eq!(
            classify_linux_session(false, Some("x11"), false),
            LinuxSession::Unavailable
        );
    }

    #[test]
    fn x11_targets_exclude_and_reject_save_targets() {
        let atoms = X11TargetAtoms {
            targets: 1,
            utf8_string: 2,
            text: 3,
            string: 4,
            text_plain_utf8: 5,
            text_plain: 6,
            kde_hint: 7,
            save_targets: 8,
        };

        assert_eq!(atoms.offered(), [1, 2, 3, 4, 5, 6, 7]);
        assert!(!atoms.offered().contains(&atoms.save_targets));
        assert_eq!(atoms.reply_for(atoms.targets), X11TargetReply::Targets);
        assert_eq!(atoms.reply_for(atoms.text), X11TargetReply::Text(2));
        assert_eq!(atoms.reply_for(atoms.utf8_string), X11TargetReply::Text(2));
        assert_eq!(atoms.reply_for(atoms.kde_hint), X11TargetReply::Hint);
        assert_eq!(atoms.reply_for(atoms.save_targets), X11TargetReply::Reject);
        assert_eq!(atoms.reply_for(99), X11TargetReply::Reject);
    }

    #[test]
    fn x11_lease_identity_requires_window_and_lease_id() {
        let active = X11LeaseToken {
            lease_id: 41,
            window: 900,
        };

        assert!(x11_token_matches(Some(active), active));
        assert!(!x11_token_matches(
            Some(active),
            X11LeaseToken {
                lease_id: 42,
                window: 900,
            }
        ));
        assert!(!x11_token_matches(None, active));
    }

    #[test]
    fn main_thread_bridge_waits_for_completion_before_ack() {
        let (scheduled_sender, scheduled_receiver) = mpsc::channel::<MainThreadTask>();
        let (result_sender, result_receiver) = mpsc::channel();

        let worker = thread::spawn(move || {
            let result = run_scheduled(|task| scheduled_sender.send(task).map_err(|_| ()), || 7);
            result_sender.send(result).unwrap();
        });

        let task = scheduled_receiver.recv().unwrap();
        assert!(matches!(
            result_receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        task();
        assert_eq!(result_receiver.recv().unwrap(), Ok(7));
        worker.join().unwrap();
    }

    #[test]
    fn main_thread_bridge_reports_schedule_failure() {
        let result = run_scheduled::<(), _>(|_| Err(()), || ());

        assert_eq!(result, Err(BridgeError::ScheduleFailed));
    }

    #[test]
    fn main_thread_bridge_reports_completion_loss() {
        let result = run_scheduled(|_task| Ok::<(), ()>(()), || 9);

        assert_eq!(result, Err(BridgeError::CompletionLost));
    }
}
