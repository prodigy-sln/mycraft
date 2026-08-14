//! Which key asks for what.
//!
//! A declaration rather than a decision, which is why it sits beside the input
//! dispatch instead of inside it: `session.rs` reads this table exactly as it
//! reads `mc_render::window`'s capture policies, and for the same reason — what a
//! key *means* is a statement the game makes, and what the client *does* about it
//! is the state machine next door. Splitting the two also keeps `session.rs`
//! inside its size limit, which is what forced the question, but the answer would
//! be the same either way.
//!
//! **Nothing outside this crate can ask the table.** [`Bindings::bound_action`] is
//! crate-visible, so a test that wanted to ask it the question the session asks it
//! is a compile error rather than a text-scan finding: an integration-test binary
//! is a separate crate. A test constructs bindings; it never interrogates them.

use mc_sim::player::PlayerAction;

use crate::session::KeyKind;

/// What one key press asks for.
///
/// Two arms because there are two kinds of answer, and the difference is the whole
/// of why the overlay's toggle cannot disturb a replay: a movement travels into
/// the simulation as part of a tick, and showing an instrument does not travel
/// anywhere at all. `PlayerAction` gains nothing for the overlay, so "the toggle
/// never reaches the simulation" is a fact about these two types rather than a
/// rule somebody has to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundAction {
    /// Something the world is asked for on the next tick.
    Player(PlayerAction),
    /// Showing or hiding the client's own debug overlay, which the world never
    /// hears about.
    Overlay,
}

/// The keys this client answers to, and what each of them asks for.
///
/// A value the session holds rather than a free function, because the overlay's
/// toggle is *remappable* and a key bound directly in code is a key no player can
/// move. Only the toggle is movable here; widening this into a binding editor is
/// another increment's work, and the declared movement rows are the same table
/// they have always been.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bindings {
    overlay_toggle: KeyKind,
}

impl Bindings {
    /// The table the client declares: W forward, S back, A strafe-left, D
    /// strafe-right, Space jump, and F3 the debug overlay.
    #[must_use]
    pub const fn declared() -> Self {
        Self {
            overlay_toggle: KeyKind::F3,
        }
    }

    /// The declared table with the overlay's toggle moved to `key`.
    ///
    /// It exists so "the overlay toggles through a binding rather than through a
    /// hardcoded key" is a claim something can be *asked*: bind the toggle
    /// elsewhere, and the key that used to toggle has to stop toggling.
    #[must_use]
    pub const fn with_overlay_toggle(key: KeyKind) -> Self {
        Self {
            overlay_toggle: key,
        }
    }

    /// What the player asked for by pressing `key`, if this table names it.
    ///
    /// It stays a *table*: what a movement does is `mc-sim`'s and what showing the
    /// overlay does is the overlay's, so nothing here decides anything a test
    /// cannot ask about. The one branch that acts on a `None` is the
    /// accumulator's.
    /// **The toggle is asked first, and it is asked by comparing keys** rather
    /// than by listing the pairs a table is known to be built with. A comparison
    /// against the row this table actually carries is what makes the binding a
    /// binding: a listing answers correctly for the two keys somebody thought of
    /// and silently answers `None` for every other key the toggle could be moved
    /// to, which is a table that ignores its own configuration.
    #[must_use]
    pub(crate) fn bound_action(&self, key: KeyKind) -> Option<BoundAction> {
        if key == self.overlay_toggle {
            return Some(BoundAction::Overlay);
        }
        match key {
            KeyKind::W => Some(BoundAction::Player(PlayerAction::Forward)),
            KeyKind::S => Some(BoundAction::Player(PlayerAction::Back)),
            KeyKind::A => Some(BoundAction::Player(PlayerAction::StrafeLeft)),
            KeyKind::D => Some(BoundAction::Player(PlayerAction::StrafeRight)),
            KeyKind::Space => Some(BoundAction::Player(PlayerAction::Jump)),
            KeyKind::F3 | KeyKind::F7 | KeyKind::Escape | KeyKind::Other => None,
        }
    }
}
