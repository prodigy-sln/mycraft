//! What a content change becomes on the device: `App`'s whole share of a reload.
//!
//! Every decision here was made in the simulation — admitted or not, what the
//! swap replaced, which sections need meshing again. This is only where those
//! answers reach a graphics queue. A child module rather than a sibling because
//! it writes the fields `App` owns.

use std::sync::Arc;

use mc_render::gpu::RendererError;
use mc_sim::simulation::PublishedContent;

use crate::notice;
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
                notice::say_reloading(clearing, &self.notices);
                self.serve(&content, layers)
            }
        }
    }

    /// Hands the device and the re-mesh worker the content a reload accepted.
    ///
    /// **A reload that moved only a declared opacity uploads nothing new.** The
    /// layers are the layers it already had; what carries the changed degree is
    /// the resolution retired below, which the next batch is packed against. So
    /// that edit costs a re-pack and neither a re-mesh nor a texture upload.
    ///
    /// **Nothing in this workspace reaches this function**, because `App` needs a
    /// window nothing here constructs. Readings of an accepted reload take their
    /// value from the `Unuploaded` its report handed over; the upload and the
    /// retirement performed with it are held by [`Unuploaded`]'s own type
    /// obligation rather than by an assertion, and that residual is recorded
    /// rather than closed.
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
        let said = format!("mycraft: {CONTENT_NOT_TAKEN_UP}: {reason}");
        self.reported_reload.say(&self.notices, &said);
    }
}
