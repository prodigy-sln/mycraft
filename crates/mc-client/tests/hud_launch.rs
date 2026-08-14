//! A HUD the content root cannot declare stops the launch, and says why.
//!
//! There is no error screen and no HUD-less mode to fall back to: rendering the
//! client in a content-load-failure state is out of scope for this feature, and
//! deliberately so. What a content author gets instead is the refusal itself,
//! naming the file, the element and the field — which is a better diagnostic
//! than a window drawing a world with a silently missing crosshair.
//!
//! Reached through the client's own scene preparation, which is the seam a test
//! can hold: no window, no device, no worker thread. The declarations are read
//! there beside the blocks, so a HUD fault fails the preparation exactly as an
//! unreadable block declaration already does.

mod support;

use std::error::Error;

use support::{TestResult, content};

/// The file the refused declaration is written into.
///
/// Distinctive on purpose: it is the needle a refusal that genuinely names the
/// declaration it could not accept has to carry, and no message that merely says
/// the HUD could not be loaded can contain it by accident.
const REFUSED_FILE: &str = "malformed-readout.toml";

/// The field the refusal has to name.
const REFUSED_FIELD: &str = "size";

/// A declaration every other field of which is well formed, stating an extent of
/// zero.
///
/// One fault and one only, so a refusal that names this file is naming it for
/// the reason this scenario is about. An `example:` namespace rather than
/// `base:`, because a fixture borrowing a shipped element's name would be the
/// test describing the engine in terms of the content it ships.
const REFUSED_DECLARATION: &str = "name = \"example:malformed-readout\"\nanchor = \"center\"\nsize = [0, 4]\ndraw = \"fill\"\n\
     color = \"#FFFFFFFF\"\n";

#[test]
fn a_content_root_whose_hud_declarations_are_refused_refuses_to_start_the_client() -> TestResult {
    let root = content::shipped_with(REFUSED_FILE, REFUSED_DECLARATION)?;

    let prepared = mc_client::startup::prepare_scene(root.path());

    let reported = match &prepared {
        Ok(_) => String::new(),
        Err(failure) => chain(failure),
    };
    assert!(
        prepared.is_err() && reported.contains(REFUSED_FILE) && reported.contains(REFUSED_FIELD),
        "a HUD declaration the model refuses has to stop the client starting and report the \
         fault, rather than starting with no HUD: a client that launched here would show a \
         player a world with no crosshair and tell nobody why. It prepared a scene: {}. It \
         reported: {reported}",
        prepared.is_ok()
    );
    Ok(())
}

/// Everything `failure` and the failures beneath it say, as one string.
///
/// The whole chain rather than the top message, because which layer quotes the
/// file and the field is an implementation choice: what the scenario is about is
/// that a person running the client is told both, not which error type carries
/// them.
fn chain(failure: &dyn Error) -> String {
    let mut said = vec![failure.to_string()];
    let mut beneath = failure.source();
    while let Some(cause) = beneath {
        said.push(cause.to_string());
        beneath = cause.source();
    }
    said.join(": ")
}
