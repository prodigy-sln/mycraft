//! The vocabulary a session decides in: one key of the keyboard, one button of
//! the mouse.
//!
//! **A vocabulary is not a decision**, which is why it is here and not beside the
//! dispatch that spends it. Both mirror the window library's own shapes without
//! naming them, and both carry a catch-all: a key or a button this client cannot
//! tell apart is one nothing can be bound to, so the library growing either
//! between versions changes nothing on this side of the seam.

/// One key of the keyboard, in the vocabulary the session decides in.
///
/// The same shape as `WindowEventKind`, and for the same reason: the catch-all
/// absorbs every key code the client cannot tell apart, so the window library
/// growing key codes between versions changes nothing on this side of the seam.
///
/// Named `KeyKind` rather than `Key` because the window library has a `Key` of
/// its own and the adapter imports both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    W,
    S,
    A,
    D,
    Space,
    /// The two function keys a debug overlay is conventionally reached by. Each
    /// is told apart because either may be *bound* to the toggle, and a key the
    /// client cannot tell apart is a key nothing can be bound to.
    F3,
    F7,
    Escape,
    Other,
}

/// One button of the mouse, in the vocabulary the session decides in.
///
/// The same shape as [`KeyKind`] and for the same reason: the catch-all absorbs
/// every button the client cannot tell apart, so the window library growing
/// buttons between versions changes nothing on this side of the seam.
///
/// Named `MouseButtonKind` rather than `MouseButton` because the window library
/// has a `MouseButton` of its own and the adapter imports both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonKind {
    Left,
    Right,
    Other,
}
