//! Input events, keys, and pointer styles.

use std::ops::BitOr;

use elegance_sys as sys;

use crate::geometry::Point;

/// Physical-pixel render dimensions reported by [`crate::ViewExt::on_size_change`].
///
/// Each dimension is the logical layout size multiplied by the window's display scale,
/// rounded up. Visual transforms (such as `scale_effect`), clipping and shadows are excluded.
/// Zero dimensions are possible; skip rendering or allocating textures for those sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

/// Hover intent (`on_hover_change`, submenus): how long the pointer rests before hover starts,
/// and lingers before it ends (seconds). Pointer enter/leave and controls' highlights don't wait.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoverDelay {
    pub enter: f64,
    pub exit: f64,
}

impl Default for HoverDelay {
    fn default() -> Self {
        Self { enter: 0.0, exit: 0.0 }
    }
}

impl HoverDelay {
    pub fn enter(mut self, seconds: f64) -> Self {
        self.enter = seconds;
        self
    }
    pub fn exit(mut self, seconds: f64) -> Self {
        self.exit = seconds;
        self
    }
    pub(crate) fn c(self) -> sys::ElgHoverDelay {
        sys::ElgHoverDelay { enter: self.enter, exit: self.exit }
    }
    pub(crate) fn from_c(d: sys::ElgHoverDelay) -> Self {
        Self { enter: d.enter, exit: d.exit }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MouseButton {
    #[default]
    Left,
    Middle,
    Right,
    Other,
}

impl MouseButton {
    pub(crate) fn from_c(value: i32) -> Self {
        match value {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => MouseButton::Other,
        }
    }
    pub(crate) fn c(self) -> i32 {
        self as i32
    }
}

macro_rules! keys {
    ($($key:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        #[repr(i32)]
        pub enum Key {
            #[default]
            Unknown,
            $($key),*
        }

        impl Key {
            const ALL: &'static [Key] = &[Key::Unknown, $(Key::$key),*];
            pub(crate) fn from_c(value: i32) -> Key {
                Self::ALL.get(value as usize).copied().unwrap_or(Key::Unknown)
            }
        }
    };
}

keys!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    Enter, Escape, Backspace, Tab, Space, ForwardDelete, Insert,
    Home, End, PageUp, PageDown, Left, Right, Up, Down,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Menu,
);

/// Modifier keys held during an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers(pub u32);

impl KeyModifiers {
    pub const NONE: KeyModifiers = KeyModifiers(0);
    pub const SHIFT: KeyModifiers = KeyModifiers(1);
    pub const CONTROL: KeyModifiers = KeyModifiers(2);
    pub const ALT: KeyModifiers = KeyModifiers(4);
    /// Cmd on macOS, the Windows key on Windows.
    pub const SUPER: KeyModifiers = KeyModifiers(8);

    pub fn has(self, modifier: KeyModifiers) -> bool {
        self.0 & modifier.0 != 0
    }

    /// The platform's shortcut modifier: Cmd on macOS, Ctrl elsewhere.
    pub fn primary() -> KeyModifiers {
        if cfg!(target_os = "macos") {
            KeyModifiers::SUPER
        } else {
            KeyModifiers::CONTROL
        }
    }
}

impl BitOr for KeyModifiers {
    type Output = KeyModifiers;
    fn bitor(self, other: KeyModifiers) -> KeyModifiers {
        KeyModifiers(self.0 | other.0)
    }
}

/// The mouse pointer's shape over a view, like CSS's `cursor` (see
/// [`crate::ViewExt::pointer_style`]). The innermost view under the pointer with a style other
/// than `Automatic` decides; with none, it's the arrow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PointerStyle {
    /// No style of its own: the views around it decide.
    #[default]
    Automatic,
    /// CSS's default.
    Arrow,
    /// A hand, over links and other things to click.
    Pointer,
    /// The I-beam, over text to select or edit.
    Text,
    Crosshair,
    Move,
    /// Busy: nothing can be done until it's over.
    Wait,
    /// Busy, but it still takes input.
    Progress,
    ResizeHorizontal,
    ResizeVertical,
    /// Corners: top-left to bottom-right.
    ResizeNwse,
    /// Corners: top-right to bottom-left.
    ResizeNesw,
    NotAllowed,
}

/// Where a trackpad gesture is. Mouse wheels have none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GesturePhase {
    /// Not part of a gesture (a mouse wheel, or a trackpad the platform doesn't follow).
    #[default]
    None,
    /// Fingers touched down (scrolls: possibly before moving, with no delta).
    Began,
    Changed,
    /// Fingers lifted.
    Ended,
    /// The system took the gesture over.
    Cancelled,
}

impl GesturePhase {
    fn from_c(value: i32) -> Self {
        match value {
            1 => GesturePhase::Began,
            2 => GesturePhase::Changed,
            3 => GesturePhase::Ended,
            4 => GesturePhase::Cancelled,
            _ => GesturePhase::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollEvent {
    /// Where the pointer is: in window coordinates for an [`crate::EventHandler`], in the view's
    /// own for [`crate::ViewExt::on_scroll`].
    pub location: Point,
    /// Always in the window's coordinates.
    pub window_location: Point,
    pub dx: f32,
    pub dy: f32,
    /// Trackpad-style fine deltas (apply directly) rather than wheel notches.
    pub precise: bool,
    /// Held while scrolling (Ctrl+wheel: zoom).
    pub modifiers: KeyModifiers,
    /// Trackpads on macOS: the fingers' gesture (began ... ended), then, after a flick, the
    /// system's momentum (`momentum` true, began ... ended) as the scroll coasts to a stop.
    pub phase: GesturePhase,
    pub momentum: bool,
    /// When it happened, in seconds on the event source's clock (for speeds between events);
    /// 0 if unknown.
    pub time: f64,
}

impl ScrollEvent {
    pub(crate) fn from_c(e: &sys::ElgScrollEvent) -> Self {
        Self {
            location: Point::from_c(e.location),
            window_location: Point::from_c(e.window_location),
            dx: e.dx,
            dy: e.dy,
            precise: e.precise,
            modifiers: KeyModifiers(e.modifiers),
            phase: GesturePhase::from_c(e.phase),
            momentum: e.momentum,
            time: e.time,
        }
    }
}

/// Pinching on a trackpad (or a touch screen).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MagnifyEvent {
    pub location: Point,
    /// Since the gesture began: 2 = twice as big.
    pub scale: f32,
    /// Since the last event, as a factor.
    pub delta: f32,
    pub phase: GesturePhase,
}

impl Default for MagnifyEvent {
    fn default() -> Self {
        Self { location: Point::default(), scale: 1.0, delta: 1.0, phase: GesturePhase::Changed }
    }
}

impl MagnifyEvent {
    pub(crate) fn from_c(e: &sys::ElgMagnifyEvent) -> Self {
        Self {
            location: Point::from_c(e.location),
            scale: e.scale,
            delta: e.delta,
            phase: GesturePhase::from_c(e.phase),
        }
    }
}

/// Turning two fingers on a trackpad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotateEvent {
    pub location: Point,
    /// Since the gesture began, clockwise.
    pub degrees: f32,
    /// Since the last event.
    pub delta: f32,
    pub phase: GesturePhase,
}

impl Default for RotateEvent {
    fn default() -> Self {
        Self { location: Point::default(), degrees: 0.0, delta: 0.0, phase: GesturePhase::Changed }
    }
}

impl RotateEvent {
    pub(crate) fn from_c(e: &sys::ElgRotateEvent) -> Self {
        Self {
            location: Point::from_c(e.location),
            degrees: e.degrees,
            delta: e.delta,
            phase: GesturePhase::from_c(e.phase),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputKind {
    /// Insert the text.
    #[default]
    Commit,
    /// An IME's in-progress text, replacing the previous one ("" ends it).
    Composition,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputEvent {
    pub kind: TextInputKind,
    pub text: String,
    /// Composition: the IME's caret, in bytes into `text`.
    pub cursor: usize,
}

impl TextInputEvent {
    pub(crate) fn from_c(e: &sys::ElgTextInputEvent) -> Self {
        Self {
            kind: if e.kind == 1 { TextInputKind::Composition } else { TextInputKind::Commit },
            text: unsafe { e.text.as_str() }.to_owned(),
            cursor: e.cursor,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyEventKind {
    #[default]
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    pub key: Key,
    pub modifiers: KeyModifiers,
    /// Auto-repeat while held.
    pub repeat: bool,
}

impl KeyEvent {
    pub(crate) fn from_c(e: &sys::ElgKeyEvent) -> Self {
        Self {
            kind: if e.kind == 1 { KeyEventKind::Up } else { KeyEventKind::Down },
            key: Key::from_c(e.key),
            modifiers: KeyModifiers(e.modifiers),
            repeat: e.repeat,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerEventKind {
    Down,
    Up,
    /// Only sent to the handler that captured the pointer.
    #[default]
    Move,
    /// The pointer was lost (e.g. the window closed); reset any pressed state.
    Cancel,
    Hover,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    /// In points: window coordinates for an [`crate::EventHandler`], the view's own for
    /// [`crate::ViewExt::on_pointer`].
    pub location: Point,
    pub button: MouseButton,
    /// 2 for the second click of a double click, ...
    pub click_count: i32,
    /// Held when the button went down / up.
    pub modifiers: KeyModifiers,
    /// Always in the window's coordinates.
    pub window_location: Point,
}

impl PointerEvent {
    pub(crate) fn from_c(e: &sys::ElgPointerEvent) -> Self {
        Self {
            kind: match e.kind {
                0 => PointerEventKind::Down,
                1 => PointerEventKind::Up,
                2 => PointerEventKind::Move,
                3 => PointerEventKind::Cancel,
                _ => PointerEventKind::Hover,
            },
            location: Point::from_c(e.location),
            button: MouseButton::from_c(e.button),
            click_count: e.click_count,
            modifiers: KeyModifiers(e.modifiers),
            window_location: Point::from_c(e.window_location),
        }
    }
}

/// A click on a view ([`crate::ViewExt::on_click`]): pressed and released inside it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClickEvent {
    /// Where it was released, in the view's own coordinates (0, 0: its top-leading corner).
    pub location: Point,
    pub window_location: Point,
    pub button: MouseButton,
    /// Held when it was released.
    pub modifiers: KeyModifiers,
    /// 2 for a double click, ...
    pub click_count: i32,
}

impl ClickEvent {
    pub(crate) fn from_c(e: &sys::ElgClickEvent) -> Self {
        Self {
            location: Point::from_c(e.location),
            window_location: Point::from_c(e.window_location),
            button: MouseButton::from_c(e.button),
            modifiers: KeyModifiers(e.modifiers),
            click_count: e.click_count,
        }
    }
}

/// Which clicks [`crate::ViewExt::on_click_with`] takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClickOptions {
    pub button: MouseButton,
    /// 2: double clicks (the first click goes on to outer handlers), 3: triple clicks, ...
    pub count: i32,
}

impl Default for ClickOptions {
    fn default() -> Self {
        Self { button: MouseButton::Left, count: 1 }
    }
}

impl ClickOptions {
    pub fn button(mut self, button: impl Into<MouseButton>) -> Self {
        self.button = button.into();
        self
    }
    pub fn count(mut self, count: i32) -> Self {
        self.count = count;
        self
    }
}

/// A drag in progress ([`crate::DragGesture`]). Locations are in the view's own coordinates;
/// `translation` is measured in the window, so a view that moves with the drag doesn't feed back
/// into it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragEvent {
    /// Where the pointer is now.
    pub location: Point,
    /// Where the press went down.
    pub start_location: Point,
    /// How far the pointer has moved since the press.
    pub translation: Point,
    /// Points per second, over the last moments of the drag.
    pub velocity: Point,
    pub button: MouseButton,
    /// Held when the press went down.
    pub modifiers: KeyModifiers,
}

impl DragEvent {
    pub(crate) fn from_c(e: &sys::ElgDragEvent) -> Self {
        Self {
            location: Point::from_c(e.location),
            start_location: Point::from_c(e.start_location),
            translation: Point::from_c(e.translation),
            velocity: Point::from_c(e.velocity),
            button: MouseButton::from_c(e.button),
            modifiers: KeyModifiers(e.modifiers),
        }
    }
}

/// What an event handler did with an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventResult {
    /// Handled: don't pass it on.
    pub consumed: bool,
    /// Draw a new frame.
    pub redraw: bool,
    /// With `consumed` on a down: gestures around the view ([`crate::ViewExt::recognize`]) may
    /// take the press over, and the handler then gets `Cancel`. For presses that are clicks;
    /// handlers that track the pointer themselves (sliders) leave it false.
    pub cancellable: bool,
}

impl EventResult {
    pub const IGNORED: EventResult = EventResult { consumed: false, redraw: false, cancellable: false };
    pub const CONSUMED: EventResult = EventResult { consumed: true, redraw: false, cancellable: false };
    pub const REDRAW: EventResult = EventResult { consumed: true, redraw: true, cancellable: false };

    /// A consumed press that gestures around the view may take over.
    pub fn cancellable(self) -> Self {
        Self { cancellable: true, ..self }
    }

    pub(crate) fn c(self) -> sys::ElgEventResult {
        sys::ElgEventResult { consumed: self.consumed, redraw: self.redraw, cancellable: self.cancellable }
    }
}
