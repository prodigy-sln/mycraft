//! What a run reports once whatever was being played has been saved.
//!
//! Its own file rather than the module above's, because it is a different reason
//! to change: everything there is about what a keystroke, a pointer motion or a
//! frame does to a running world, and this is about the one moment there is no
//! longer one.

use std::path::Path;

use mc_render::window::Ending;

use crate::session::Session;

/// The ending a run reports once whatever was being played has been saved.
///
/// **Only a run that ended by closing normally saves.** A device-lost run is not
/// a clean quit, and treating it as one would let a broken frame path overwrite
/// a good world. A failed save on a clean close becomes a failed ending naming
/// the path and the reason; a save failure never masks an ending that was
/// already a failure.
///
/// It lives beside the session rather than in the simulation because it answers
/// in the window's own vocabulary, and the simulation may not name the renderer.
#[must_use]
pub fn ending_after_saving(session: Option<&Session>, ending: Ending, save: &Path) -> Ending {
    if !matches!(ending, Ending::Closed) {
        return ending;
    }
    match session.map_or(Ok(()), |playing| playing.save(save)) {
        Ok(()) => ending,
        Err(refused) => Ending::failed_under(
            &format!(
                "the world could not be saved to {path}",
                path = save.display()
            ),
            &refused,
        ),
    }
}
