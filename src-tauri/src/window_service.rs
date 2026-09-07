use std::{error::Error, fmt, time::Duration};

use tauri::{LogicalPosition, LogicalSize, WebviewWindow};

pub const MAIN_WIDTH: f64 = 400.0;
pub const ROLLED_HEIGHT: f64 = 40.0;
pub const ABOUT_WIDTH: f64 = 360.0;
pub const ABOUT_HEIGHT: f64 = 400.0;
pub const ABOUT_GAP: f64 = 14.0;
pub const REDACTION_DELAY: Duration = Duration::from_millis(250);
pub const EXIT_CLEANUP_DELAY: Duration = Duration::from_secs(2);
pub const MAX_JAVASCRIPT_INTEGER: u64 = 9_007_199_254_740_991;

const MAIN_LABEL: &str = "main";
const ABOUT_LABEL: &str = "about";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum View {
    Expanded,
    Rolled,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl LogicalBounds {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug)]
pub enum WindowServiceError {
    InvalidContentHeight,
    InvalidGeometry,
    MonitorUnavailable,
    UnexpectedWindowLabel {
        expected: &'static str,
        actual: String,
    },
    Native(tauri::Error),
}

impl fmt::Display for WindowServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContentHeight => formatter.write_str("invalid measured content height"),
            Self::InvalidGeometry => formatter.write_str("invalid window geometry"),
            Self::MonitorUnavailable => formatter.write_str("current monitor is unavailable"),
            Self::UnexpectedWindowLabel { expected, actual } => {
                write!(formatter, "expected {expected} window, got {actual}")
            }
            Self::Native(error) => write!(formatter, "native window operation failed: {error}"),
        }
    }
}

impl Error for WindowServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Native(error) => Some(error),
            _ => None,
        }
    }
}

impl From<tauri::Error> for WindowServiceError {
    fn from(error: tauri::Error) -> Self {
        Self::Native(error)
    }
}

pub fn main_bounds(
    view: View,
    measured_content_height: f64,
    zoom_percent: i32,
    current_x: f64,
    current_y: f64,
    work_area: LogicalBounds,
) -> Result<LogicalBounds, WindowServiceError> {
    validate_content_height(measured_content_height)?;
    validate_zoom_percent(zoom_percent)?;
    let zoom = f64::from(zoom_percent.max(100));
    let width = MAIN_WIDTH * zoom / 100.0;
    let height = match view {
        View::Expanded => measured_content_height.ceil(),
        View::Rolled => ROLLED_HEIGHT * zoom / 100.0,
    };
    clamp_bounds(
        LogicalBounds::new(current_x, current_y, width, height),
        work_area,
    )
}

pub fn about_bounds(
    main: LogicalBounds,
    work_area: LogicalBounds,
) -> Result<LogicalBounds, WindowServiceError> {
    validate_bounds(main)?;
    validate_bounds(work_area)?;

    let width = ABOUT_WIDTH.min(work_area.width);
    let height = ABOUT_HEIGHT.min(work_area.height);
    let candidates = [
        LogicalBounds::new(main.x + main.width + ABOUT_GAP, main.y, width, height),
        LogicalBounds::new(main.x - ABOUT_GAP - width, main.y, width, height),
        LogicalBounds::new(main.x, main.y + main.height + ABOUT_GAP, width, height),
        LogicalBounds::new(main.x, main.y - ABOUT_GAP - height, width, height),
    ];

    for candidate in candidates {
        let candidate = clamp_bounds(candidate, work_area)?;
        if !overlaps(candidate, main) {
            return Ok(candidate);
        }
    }

    // Overlap is unavoidable when no side can fit both windows in the work area.
    clamp_bounds(candidates[0], work_area)
}

pub fn set_main_view(
    window: &WebviewWindow,
    view: View,
    measured_content_height: f64,
    zoom_percent: i32,
) -> Result<LogicalBounds, WindowServiceError> {
    require_label(window, MAIN_LABEL)?;
    let (scale_factor, work_area) = current_work_area(window)?;
    let position = window.outer_position()?.to_logical::<f64>(scale_factor);
    let bounds = main_bounds(
        view,
        measured_content_height,
        zoom_percent,
        position.x,
        position.y,
        work_area,
    )?;
    apply_bounds(window, bounds)?;
    Ok(bounds)
}

pub fn set_main_topmost(window: &WebviewWindow, enabled: bool) -> Result<(), WindowServiceError> {
    require_label(window, MAIN_LABEL)?;
    window.set_always_on_top(enabled)?;
    Ok(())
}

pub fn start_window_drag(window: &WebviewWindow) -> Result<(), WindowServiceError> {
    if !matches!(window.label(), MAIN_LABEL | ABOUT_LABEL) {
        return Err(WindowServiceError::UnexpectedWindowLabel {
            expected: "main or about",
            actual: window.label().to_owned(),
        });
    }
    window.start_dragging()?;
    Ok(())
}

pub fn place_about(
    main: &WebviewWindow,
    about: &WebviewWindow,
) -> Result<LogicalBounds, WindowServiceError> {
    require_label(main, MAIN_LABEL)?;
    require_label(about, ABOUT_LABEL)?;

    let (scale_factor, work_area) = current_work_area(main)?;
    let main_position = main.outer_position()?.to_logical::<f64>(scale_factor);
    let main_size = main.outer_size()?.to_logical::<f64>(scale_factor);
    let main_bounds = LogicalBounds::new(
        main_position.x,
        main_position.y,
        main_size.width,
        main_size.height,
    );
    let bounds = about_bounds(main_bounds, work_area)?;
    apply_bounds_at_scale(about, bounds, scale_factor)?;
    Ok(bounds)
}

pub fn validate_zoom_percent(percent: i32) -> Result<(), WindowServiceError> {
    if matches!(percent, 75 | 90 | 100 | 110 | 125 | 150 | 175 | 200) {
        Ok(())
    } else {
        Err(WindowServiceError::InvalidGeometry)
    }
}

fn validate_content_height(height: f64) -> Result<(), WindowServiceError> {
    if height.is_finite() && height > 0.0 {
        Ok(())
    } else {
        Err(WindowServiceError::InvalidContentHeight)
    }
}

fn validate_bounds(bounds: LogicalBounds) -> Result<(), WindowServiceError> {
    let right = bounds.x + bounds.width;
    let bottom = bounds.y + bounds.height;
    if bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0
        && right.is_finite()
        && bottom.is_finite()
    {
        Ok(())
    } else {
        Err(WindowServiceError::InvalidGeometry)
    }
}

fn overlaps(first: LogicalBounds, second: LogicalBounds) -> bool {
    first.x < second.x + second.width
        && first.x + first.width > second.x
        && first.y < second.y + second.height
        && first.y + first.height > second.y
}

fn clamp_bounds(
    requested: LogicalBounds,
    work_area: LogicalBounds,
) -> Result<LogicalBounds, WindowServiceError> {
    validate_bounds(requested)?;
    validate_bounds(work_area)?;

    let width = requested.width.min(work_area.width);
    let height = requested.height.min(work_area.height);
    let x = requested
        .x
        .clamp(work_area.x, work_area.x + work_area.width - width);
    let y = requested
        .y
        .clamp(work_area.y, work_area.y + work_area.height - height);
    Ok(LogicalBounds::new(x, y, width, height))
}

fn current_work_area(window: &WebviewWindow) -> Result<(f64, LogicalBounds), WindowServiceError> {
    let monitor = window
        .current_monitor()?
        .ok_or(WindowServiceError::MonitorUnavailable)?;
    let scale_factor = monitor.scale_factor();
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(WindowServiceError::InvalidGeometry);
    }

    let work_area = monitor.work_area();
    let position = work_area.position.to_logical::<f64>(scale_factor);
    let size = work_area.size.to_logical::<f64>(scale_factor);
    let bounds = LogicalBounds::new(position.x, position.y, size.width, size.height);
    validate_bounds(bounds)?;
    Ok((scale_factor, bounds))
}

fn apply_bounds(window: &WebviewWindow, bounds: LogicalBounds) -> Result<(), WindowServiceError> {
    validate_bounds(bounds)?;
    window.set_size(LogicalSize::new(bounds.width, bounds.height))?;
    window.set_position(LogicalPosition::new(bounds.x, bounds.y))?;
    Ok(())
}

fn apply_bounds_at_scale(
    window: &WebviewWindow,
    bounds: LogicalBounds,
    scale_factor: f64,
) -> Result<(), WindowServiceError> {
    validate_bounds(bounds)?;
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(WindowServiceError::InvalidGeometry);
    }

    let size = LogicalSize::new(bounds.width, bounds.height).to_physical::<u32>(scale_factor);
    let position = LogicalPosition::new(bounds.x, bounds.y).to_physical::<i32>(scale_factor);
    window.set_size(size)?;
    window.set_position(position)?;
    Ok(())
}

fn require_label(window: &WebviewWindow, expected: &'static str) -> Result<(), WindowServiceError> {
    if window.label() == expected {
        Ok(())
    } else {
        Err(WindowServiceError::UnexpectedWindowLabel {
            expected,
            actual: window.label().to_owned(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    Main,
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RollToken(u64);

impl RollToken {
    #[cfg(test)]
    pub const fn generation(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewAction {
    Unchanged,
    Roll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerAction {
    Unchanged,
    Cancel,
    Defer { token: RollToken },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    pub opacity: f64,
    pub view: ViewAction,
    pub timer: TimerAction,
}

#[derive(Debug, Default)]
pub struct Coordinator {
    main_focused: bool,
    about_focused: bool,
    main_pointer_inside: bool,
    about_pointer_inside: bool,
    about_open: bool,
    high_contrast: bool,
    rolled: bool,
    next_generation: u64,
    pending_roll: Option<RollToken>,
}

impl Coordinator {
    pub fn set_focus(&mut self, surface: Surface, focused: bool) -> Effects {
        if surface == Surface::About && !self.about_open {
            return self.steady();
        }
        let target = match surface {
            Surface::Main => &mut self.main_focused,
            Surface::About => &mut self.about_focused,
        };
        if *target == focused {
            return self.steady();
        }
        *target = focused;
        self.reconcile()
    }

    pub fn set_pointer_inside(&mut self, surface: Surface, inside: bool) -> Effects {
        if surface == Surface::About && !self.about_open {
            return self.steady();
        }
        let target = match surface {
            Surface::Main => &mut self.main_pointer_inside,
            Surface::About => &mut self.about_pointer_inside,
        };
        if *target == inside {
            return self.steady();
        }
        *target = inside;
        self.steady()
    }

    pub fn set_about_open(&mut self, open: bool) -> Effects {
        if self.about_open == open {
            return self.steady();
        }
        self.about_open = open;
        if !open {
            self.about_focused = false;
            self.about_pointer_inside = false;
        }
        self.reconcile()
    }

    pub fn set_high_contrast(&mut self, enabled: bool) -> Effects {
        if self.high_contrast == enabled {
            return self.steady();
        }
        self.high_contrast = enabled;
        self.reconcile()
    }

    pub fn note_view(&mut self, view: View) {
        self.rolled = view == View::Rolled;
    }

    pub fn roll_due(&mut self, token: RollToken) -> Effects {
        if self.pending_roll != Some(token) || !self.roll_eligible() {
            return self.steady();
        }
        self.pending_roll = None;
        self.rolled = true;
        Effects {
            opacity: self.opacity(),
            view: ViewAction::Roll,
            timer: TimerAction::Unchanged,
        }
    }

    #[cfg(test)]
    pub const fn pending_roll(&self) -> Option<RollToken> {
        self.pending_roll
    }

    #[cfg(test)]
    pub const fn is_rolled(&self) -> bool {
        self.rolled
    }

    pub const fn is_high_contrast(&self) -> bool {
        self.high_contrast
    }

    fn reconcile(&mut self) -> Effects {
        let eligible = self.roll_eligible();
        let timer = if eligible {
            match self.pending_roll {
                Some(_) => TimerAction::Unchanged,
                None => {
                    let token = self.next_token();
                    self.pending_roll = Some(token);
                    TimerAction::Defer { token }
                }
            }
        } else if self.pending_roll.take().is_some() {
            TimerAction::Cancel
        } else {
            TimerAction::Unchanged
        };
        Effects {
            opacity: self.opacity(),
            view: ViewAction::Unchanged,
            timer,
        }
    }

    fn next_token(&mut self) -> RollToken {
        self.next_generation = self.next_generation.wrapping_add(1);
        if self.next_generation == 0 {
            self.next_generation = 1;
        }
        RollToken(self.next_generation)
    }

    fn roll_eligible(&self) -> bool {
        !self.high_contrast && !self.active() && !self.about_open && !self.rolled
    }

    fn active(&self) -> bool {
        self.main_focused || (self.about_open && self.about_focused)
    }

    fn hovered(&self) -> bool {
        self.main_pointer_inside || (self.about_open && self.about_pointer_inside)
    }

    fn opacity(&self) -> f64 {
        if self.high_contrast || self.active() {
            1.0
        } else if self.hovered() {
            0.95
        } else {
            0.85
        }
    }

    fn steady(&self) -> Effects {
        Effects {
            opacity: self.opacity(),
            view: ViewAction::Unchanged,
            timer: TimerAction::Unchanged,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseStart {
    AlreadyClosing,
    DestroyNow,
    AwaitRedaction { ticket: u64, after: Duration },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedactionAction {
    Stale,
    Unchanged,
    DestroyNow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitAction {
    Allow,
    StartBoundedClose,
    Prevent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClosePhase {
    Running,
    AwaitingRedaction(u64),
    ReadyToDestroy,
    Headless,
    ExitAllowed,
}

#[derive(Debug)]
pub struct CloseReducer {
    phase: ClosePhase,
    next_ticket: u64,
}

impl Default for CloseReducer {
    fn default() -> Self {
        Self {
            phase: ClosePhase::Running,
            next_ticket: 0,
        }
    }
}

impl CloseReducer {
    pub fn begin_frontend(&mut self) -> CloseStart {
        if self.phase != ClosePhase::Running {
            return CloseStart::AlreadyClosing;
        }
        self.phase = ClosePhase::ReadyToDestroy;
        CloseStart::DestroyNow
    }

    pub fn begin_native(&mut self) -> CloseStart {
        if self.phase != ClosePhase::Running {
            return CloseStart::AlreadyClosing;
        }
        let ticket = self.next_ticket();
        self.phase = ClosePhase::AwaitingRedaction(ticket);
        CloseStart::AwaitRedaction {
            ticket,
            after: REDACTION_DELAY,
        }
    }

    pub fn acknowledge(&mut self, ticket: u64) -> RedactionAction {
        match self.phase {
            ClosePhase::AwaitingRedaction(expected) if expected == ticket => {
                self.phase = ClosePhase::ReadyToDestroy;
                RedactionAction::DestroyNow
            }
            ClosePhase::AwaitingRedaction(_) => RedactionAction::Stale,
            _ => RedactionAction::Stale,
        }
    }

    pub fn redaction_due(&mut self, ticket: u64) -> RedactionAction {
        if self.phase == ClosePhase::AwaitingRedaction(ticket) {
            self.phase = ClosePhase::ReadyToDestroy;
            RedactionAction::DestroyNow
        } else {
            RedactionAction::Unchanged
        }
    }

    pub fn mark_destroyed(&mut self) -> bool {
        if self.phase != ClosePhase::ReadyToDestroy {
            return false;
        }
        self.phase = ClosePhase::Headless;
        true
    }

    pub const fn exit_requested(&self) -> ExitAction {
        match self.phase {
            ClosePhase::ExitAllowed => ExitAction::Allow,
            ClosePhase::Running => ExitAction::StartBoundedClose,
            _ => ExitAction::Prevent,
        }
    }

    pub fn allow_exit(&mut self) -> bool {
        if matches!(self.phase, ClosePhase::Running | ClosePhase::ExitAllowed) {
            return false;
        }
        self.phase = ClosePhase::ExitAllowed;
        true
    }

    pub const fn is_closing(&self) -> bool {
        !matches!(self.phase, ClosePhase::Running)
    }

    pub const fn is_destroyed(&self) -> bool {
        matches!(self.phase, ClosePhase::Headless | ClosePhase::ExitAllowed)
    }

    fn next_ticket(&mut self) -> u64 {
        self.next_ticket = if self.next_ticket >= MAX_JAVASCRIPT_INTEGER {
            1
        } else {
            self.next_ticket + 1
        };
        self.next_ticket
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const WORK_AREA: LogicalBounds = LogicalBounds::new(0.0, 0.0, 1920.0, 1080.0);

    #[test]
    fn validates_and_clamps_main_sizes() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
            assert!(matches!(
                main_bounds(View::Expanded, invalid, 100, 0.0, 0.0, WORK_AREA),
                Err(WindowServiceError::InvalidContentHeight)
            ));
        }

        let small_area = LogicalBounds::new(10.0, 20.0, 400.0, 300.0);
        let expanded = main_bounds(View::Expanded, 700.2, 100, 900.0, 900.0, small_area).unwrap();
        assert_eq!(expanded, LogicalBounds::new(10.0, 20.0, 400.0, 300.0));

        let rolled = main_bounds(View::Rolled, 700.2, 100, 50.0, 500.0, small_area).unwrap();
        assert_eq!(rolled, LogicalBounds::new(10.0, 280.0, 400.0, 40.0));
    }

    #[test]
    fn zoom_scales_both_widths_and_rolled_height() {
        for (zoom, width, rolled_height) in [
            (75, 400.0, 40.0),
            (90, 400.0, 40.0),
            (100, 400.0, 40.0),
            (110, 440.0, 44.0),
            (125, 500.0, 50.0),
            (150, 600.0, 60.0),
            (175, 700.0, 70.0),
            (200, 800.0, 80.0),
        ] {
            assert_eq!(
                main_bounds(View::Expanded, 640.2, zoom, 100.0, 120.0, WORK_AREA).unwrap(),
                LogicalBounds::new(100.0, 120.0, width, 641.0)
            );
            assert_eq!(
                main_bounds(View::Rolled, 640.2, zoom, 100.0, 120.0, WORK_AREA).unwrap(),
                LogicalBounds::new(100.0, 120.0, width, rolled_height)
            );
        }
    }

    #[test]
    fn unsupported_zoom_values_are_rejected_for_both_views() {
        for zoom in [
            i32::MIN,
            -1,
            0,
            74,
            76,
            99,
            101,
            124,
            126,
            149,
            199,
            201,
            i32::MAX,
        ] {
            for view in [View::Expanded, View::Rolled] {
                assert!(matches!(
                    main_bounds(view, 640.0, zoom, 0.0, 0.0, WORK_AREA),
                    Err(WindowServiceError::InvalidGeometry)
                ));
            }
        }
    }

    #[test]
    fn zoomed_bounds_clamp_negative_work_areas_and_keep_fitting_anchor() {
        let small_area = LogicalBounds::new(-1200.0, -800.0, 600.0, 300.0);
        assert_eq!(
            main_bounds(View::Expanded, 700.2, 200, -1100.0, -750.0, small_area).unwrap(),
            small_area
        );
        assert_eq!(
            main_bounds(View::Rolled, 700.2, 200, -1100.0, -750.0, small_area).unwrap(),
            LogicalBounds::new(-1200.0, -750.0, 600.0, 80.0)
        );
        let short_area = LogicalBounds::new(-600.0, -100.0, 500.0, 60.0);
        assert_eq!(
            main_bounds(View::Rolled, 700.2, 200, -100.0, 0.0, short_area).unwrap(),
            short_area
        );
        let large_area = LogicalBounds::new(-1920.0, -1080.0, 1920.0, 1080.0);
        assert_eq!(
            main_bounds(View::Expanded, 600.2, 200, -1500.0, -900.0, large_area).unwrap(),
            LogicalBounds::new(-1500.0, -900.0, 800.0, 601.0)
        );
    }

    #[test]
    fn about_placement_tries_right_left_below_then_above() {
        let right = about_bounds(
            LogicalBounds::new(100.0, 120.0, MAIN_WIDTH, 600.0),
            WORK_AREA,
        )
        .unwrap();
        assert_eq!(
            right,
            LogicalBounds::new(100.0 + MAIN_WIDTH + ABOUT_GAP, 120.0, 360.0, 400.0)
        );

        let left = about_bounds(
            LogicalBounds::new(1500.0, 900.0, MAIN_WIDTH, 600.0),
            WORK_AREA,
        )
        .unwrap();
        assert_eq!(left, LogicalBounds::new(1126.0, 680.0, 360.0, 400.0));

        let narrow_work_area = LogicalBounds::new(0.0, 0.0, 700.0, 1080.0);
        let below = about_bounds(
            LogicalBounds::new(150.0, 100.0, MAIN_WIDTH, 300.0),
            narrow_work_area,
        )
        .unwrap();
        assert_eq!(below, LogicalBounds::new(150.0, 414.0, 360.0, 400.0));

        let above = about_bounds(
            LogicalBounds::new(150.0, 780.0, MAIN_WIDTH, 300.0),
            narrow_work_area,
        )
        .unwrap();
        assert_eq!(above, LogicalBounds::new(150.0, 366.0, 360.0, 400.0));
    }

    #[test]
    fn about_overlaps_only_when_no_side_can_fit() {
        let work_area = LogicalBounds::new(0.0, 0.0, 400.0, 300.0);
        let main = LogicalBounds::new(0.0, 0.0, 400.0, 300.0);
        assert_eq!(
            about_bounds(main, work_area).unwrap(),
            LogicalBounds::new(40.0, 0.0, 360.0, 300.0)
        );
    }

    #[test]
    fn outside_focus_defers_once_and_stale_tokens_cannot_roll() {
        let mut coordinator = Coordinator::default();
        assert_eq!(coordinator.set_focus(Surface::Main, true).opacity, 1.0);

        let first = match coordinator.set_focus(Surface::Main, false).timer {
            TimerAction::Defer { token } => token,
            action => panic!("expected deferral, got {action:?}"),
        };
        assert_eq!(
            coordinator.set_focus(Surface::Main, true).timer,
            TimerAction::Cancel
        );
        assert_eq!(coordinator.roll_due(first).view, ViewAction::Unchanged);

        let second = match coordinator.set_focus(Surface::Main, false).timer {
            TimerAction::Defer { token } => token,
            action => panic!("expected new deferral, got {action:?}"),
        };
        assert_ne!(first.generation(), second.generation());
        assert_eq!(coordinator.roll_due(second).view, ViewAction::Roll);
        assert!(coordinator.is_rolled());
        assert_eq!(
            coordinator.set_focus(Surface::Main, true).view,
            ViewAction::Unchanged
        );
        assert!(coordinator.is_rolled());
    }

    #[test]
    fn hover_changes_only_opacity_and_never_expands() {
        let mut coordinator = Coordinator::default();
        assert_eq!(
            coordinator.set_pointer_inside(Surface::Main, true),
            Effects {
                opacity: 0.95,
                view: ViewAction::Unchanged,
                timer: TimerAction::Unchanged,
            }
        );
        assert_eq!(
            coordinator.set_pointer_inside(Surface::Main, false).opacity,
            0.85
        );
        assert!(coordinator.pending_roll().is_none());

        coordinator.note_view(View::Rolled);
        assert_eq!(
            coordinator.set_pointer_inside(Surface::Main, true).view,
            ViewAction::Unchanged
        );
        assert!(coordinator.is_rolled());
    }

    #[test]
    fn about_focus_handoff_cancels_outside_collapse() {
        let mut coordinator = Coordinator::default();
        coordinator.set_focus(Surface::Main, true);
        coordinator.set_about_open(true);
        assert_eq!(
            coordinator.set_focus(Surface::Main, false).timer,
            TimerAction::Unchanged
        );
        coordinator.set_focus(Surface::About, true);

        let pending = match coordinator.set_about_open(false).timer {
            TimerAction::Defer { token } => token,
            action => panic!("expected deferral, got {action:?}"),
        };
        assert_eq!(
            coordinator.set_focus(Surface::Main, true).timer,
            TimerAction::Cancel
        );
        assert_eq!(coordinator.roll_due(pending).view, ViewAction::Unchanged);
    }

    #[test]
    fn high_contrast_suppresses_outside_collapse() {
        let mut coordinator = Coordinator::default();
        coordinator.set_focus(Surface::Main, true);
        let pending = match coordinator.set_focus(Surface::Main, false).timer {
            TimerAction::Defer { token } => token,
            action => panic!("expected deferral, got {action:?}"),
        };
        let high_contrast = coordinator.set_high_contrast(true);
        assert_eq!(high_contrast.opacity, 1.0);
        assert_eq!(high_contrast.timer, TimerAction::Cancel);
        assert!(coordinator.pending_roll().is_none());
        assert_eq!(coordinator.roll_due(pending).view, ViewAction::Unchanged);
    }
    #[test]
    fn close_reducer_rejects_stale_ack_and_allows_only_guarded_exit() {
        let mut close = CloseReducer::default();
        assert_eq!(close.exit_requested(), ExitAction::StartBoundedClose);
        let ticket = match close.begin_native() {
            CloseStart::AwaitRedaction { ticket, after } => {
                assert_eq!(after, REDACTION_DELAY);
                ticket
            }
            action => panic!("expected native redaction, got {action:?}"),
        };

        assert_eq!(close.exit_requested(), ExitAction::Prevent);
        assert_eq!(close.begin_native(), CloseStart::AlreadyClosing);
        assert_eq!(close.acknowledge(ticket + 1), RedactionAction::Stale);
        assert_eq!(close.redaction_due(ticket), RedactionAction::DestroyNow);
        assert!(close.mark_destroyed());
        assert!(!close.mark_destroyed());
        assert_eq!(close.exit_requested(), ExitAction::Prevent);
        assert!(close.allow_exit());
        assert_eq!(close.exit_requested(), ExitAction::Allow);
    }

    #[test]
    fn frontend_close_is_ready_to_destroy_without_an_ack() {
        let mut close = CloseReducer::default();
        assert_eq!(close.begin_frontend(), CloseStart::DestroyNow);
        assert!(close.is_closing());
        assert_eq!(close.exit_requested(), ExitAction::Prevent);
        assert!(!close.is_destroyed());
        assert!(close.mark_destroyed());
        assert!(close.is_destroyed());
        assert_eq!(close.acknowledge(1), RedactionAction::Stale);
    }
}
