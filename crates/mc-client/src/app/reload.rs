//! What a content change becomes on the device: `App`'s whole share of a reload.
//!
//! Every decision here was made in the simulation — admitted or not, what the
//! swap replaced, which sections need meshing again. This is only where those
//! answers reach a graphics queue. A child module rather than a sibling because
//! it writes the fields `App` owns.

use std::sync::Arc;

use mc_render::gpu::RendererError;
use mc_sim::simulation::PublishedContent;
use mc_sim::world::Clearing;

use crate::session::Session;
use crate::session::reload::{CONTENT_NOT_TAKEN_UP, ReloadReport};
use crate::upload::Unuploaded;

use super::App;

impl App {
    /// Applies whatever the last tick boundary made of the content root.
    ///
    /// **A failed texture upload after an accepted swap ends the run**, on the
    /// grounds `PreparationError::Upload` already does: the simulation would be
    /// serving content the device never received. `report_remesh`'s draw-on trade
    /// is right for a *batch* — a stale section is a stale picture of the same
    /// content — and wrong here, because the content itself has moved.
    ///
    /// This governs the texture upload only; a scene that will not pack still goes
    /// through `show`.
    ///
    /// # Errors
    ///
    /// Returns whatever the texture upload refused.
    pub(super) fn take_up_reloaded_content(
        &mut self,
        session: &mut Session,
    ) -> Result<(), RendererError> {
        match session.take_reload_report() {
            None => Ok(()),
            Some(ReloadReport::Refused(said)) => {
                self.report_reload(&said);
                Ok(())
            }
            Some(ReloadReport::Accepted {
                content,
                layers,
                clearing,
            }) => {
                report_clearing(clearing);
                self.serve(&content, layers)
            }
        }
    }

    /// Hands the device and the re-mesh worker the content a reload accepted.
    ///
    /// # Errors
    ///
    /// Returns whatever the texture upload refused.
    fn serve(
        &mut self,
        content: &PublishedContent,
        layers: Unuploaded,
    ) -> Result<(), RendererError> {
        let uploaded = layers.uploaded_to(&mut self.renderer, &self.gpu.queue)?;
        // Told on the same ordered channel its batches use, so the next batch is
        // packed against these layers without a handshake. The value only exists
        // because the upload above produced it.
        if let Some(remesher) = self.remesher.as_mut() {
            remesher.retire(uploaded, content.serial);
        }
        self.hud = Arc::clone(&content.hud);
        Ok(())
    }

    /// Says why a content root was turned away, once per distinct refusal.
    ///
    /// The simulation already reports a refusal once per save; this field is what
    /// stops the frame path repeating it every frame afterwards.
    fn report_reload(&mut self, reason: &str) {
        if self.reported_reload.as_deref() != Some(reason) {
            eprintln!("mycraft: {CONTENT_NOT_TAKEN_UP}: {reason}");
            self.reported_reload = Some(reason.to_owned());
        }
    }
}

/// Tells the player what the swap did to where they were standing.
///
/// **`NoClearSpaceWithin` is the one a person must hear**: the reload stood, they
/// are still inside solid rock, and nothing else in the run would say so. The move
/// is said too, because being teleported without explanation reads as a bug. Said
/// once per reload rather than deduplicated: each of these is one event, not a
/// condition that recurs every frame.
fn report_clearing(clearing: Clearing) {
    match clearing {
        Clearing::Unneeded => {}
        Clearing::MovedTo(feet) => {
            eprintln!(
                "mycraft: the reload made your cell solid, so you were moved to \
                 ({x}, {y}, {z})",
                x = feet.x,
                y = feet.y,
                z = feet.z
            );
        }
        Clearing::NoClearSpaceWithin { blocks } => {
            eprintln!(
                "mycraft: the reload made your cell solid and nothing within {blocks} blocks is \
                 clear, so you were left where you were"
            );
        }
    }
}
