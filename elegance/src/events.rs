//! Input events, keys, and cursors.

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

/// How long the pointer rests before hover starts, and lingers before it ends (seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoverDelay {
    pub enter: f64,
    pub exit: f64,
}

impl Default for HoverDelay {
    fn default() -> Self {
        Self { enter: 0.1, exit: 0.0 }
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
    fn from_c(value: i32) -> Self {
        match value {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => MouseButton::Other,
        }
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

/// The mouse pointer's shape (see [`crate::set_cursor`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cursor {
    #[default]
    Arrow,
    /// The I-beam.
    Text,
    /// A hand, over links.
    Pointer,
    Crosshair,
    Move,
    ResizeHorizontal,
    ResizeVertical,
    /// Corners: top-left to bottom-right.
    ResizeNwse,
    /// Corners: top-right to bottom-left.
    ResizeNesw,
    NotAllowed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollEvent {
    pub location: Point,
    pub dx: f32,
    pub dy: f32,
    /// Trackpad-style fine deltas (apply directly) rather than wheel notches.
    pub precise: bool,
    /// Held while scrolling (Ctrl+wheel: zoom).
    pub modifiers: KeyModifiers,
}

impl ScrollEvent {
    pub(crate) fn from_c(e: &sys::ElgScrollEvent) -> Self {
        Self {
            location: Point::from_c(e.location),
            dx: e.dx,
            dy: e.dy,
            precise: e.precise,
            modifiers: KeyModifiers(e.modifiers),
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
    /// Window coordinates, in points.
    pub location: Point,
    pub button: MouseButton,
    /// 2 for the second click of a double click, ...
    pub click_count: i32,
    /// Held when the button went down / up.
    pub modifiers: KeyModifiers,
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
        }
    }
}

/// A left-button drag over a view. Movement is measured in window points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragEvent {
    Started,
    /// Movement since the previous pointer position, not since the drag started.
    Moved {
        delta: Point,
    },
    /// The button was released or the drag was cancelled.
    Ended,
}

/// What an event handler did with an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventResult {
    /// Handled: don't pass it on.
    pub consumed: bool,
    /// Draw a new frame.
    pub redraw: bool,
}

impl EventResult {
    pub const IGNORED: EventResult = EventResult { consumed: false, redraw: false };
    pub const CONSUMED: EventResult = EventResult { consumed: true, redraw: false };
    pub const REDRAW: EventResult = EventResult { consumed: true, redraw: true };

    pub(crate) fn c(self) -> sys::ElgEventResult {
        sys::ElgEventResult { consumed: self.consumed, redraw: self.redraw }
    }
}
