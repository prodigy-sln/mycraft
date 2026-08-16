//! What a content author reads when the client will not accept their HUD.
//!
//! The same three questions the block declarations are asked, in the same terms:
//! which file, which element, which field. A HUD declaration is content exactly
//! as a block is, and the two kinds of content a person can write behave the same
//! way when they get one wrong.
//!
//! There is no error screen and no HUD-less mode to fall back to: rendering the
//! client in a content-load-failure state is out of scope, and deliberately so. A
//! window that opened with a silently missing crosshair would tell nobody why.
//!
//! **The assertion is on what is written, never on what is walked.** An earlier
//! version of this file walked the failure's causes itself, joined them with its
//! own separator, and asserted against its own walk — so it passed while the
//! client printed one sentence and threw the rest away. A suite that renders a
//! refusal of its own agrees with itself; only the text the shipped reporting put
//! into a sink says anything about what a person reads.
//!
//! Reached through the client's own scene preparation, which is the seam a test
//! can hold: no window, no device, no worker thread.

mod support;

use std::error::Error;
use std::path::Path;

use mc_core::hud::HudLoadError;
use mc_core::hud::source::HudElementSourceError;
use mc_render::window::Ending;

use support::{TestResult, content};

/// The file the refused declaration is written into.
///
/// Distinctive on purpose: it is the needle a refusal that genuinely names the
/// declaration it could not accept has to carry, and no message that merely says
/// the HUD could not be read can hold it by accident.
const REFUSED_FILE: &str = "malformed-readout.toml";

/// The name the refused declaration gives itself, and the field at fault in it.
///
/// The element name is asked for here and was not before: this file's own header
/// has always claimed the refusal names the file, the element and the field, and
/// only two of the three were ever checked.
const REFUSED_ELEMENT: &str = "example:malformed-readout";
const REFUSED_FIELD: &str = "size";

/// A declaration every other field of which is well formed, stating an extent of
/// zero.
///
/// One fault and one only, so a refusal that names this file is naming it for the
/// reason this scenario is about. An `example:` namespace rather than `base:`,
/// because a fixture borrowing a shipped element's name would be the test
/// describing the engine in terms of the content it ships.
const REFUSED_DECLARATION: &str = "name = \"example:malformed-readout\"\nanchor = \"center\"\nsize = [0, 4]\ndraw = \"fill\"\n\
     color = \"#FFFFFFFF\"\n";

/// A file that is not TOML at all, so there is no field to read a name out of and
/// the whole file is what is wrong.
const NOT_TOML: &str = "this is not toml\n";

/// How a refusal spells the element it is about, and how it spells the field.
///
/// Named here so that the scenario requiring *no* element and *no* field has
/// something to look for the absence of that a path or a sentence cannot hold by
/// accident — unlike the bare words, which the prose around them contains.
const ELEMENT_CLAUSE: &str = ", element `";
const FIELD_CLAUSE: &str = ", field `";

/// What the client says above a refusal that came out of the HUD declarations:
/// the layer the rest of the chain hangs from.
const HUD_REFUSAL: &str = "the shipped HUD declarations could not be read";

#[test]
fn a_refused_hud_declaration_names_the_file_the_element_and_the_field() -> TestResult {
    let root = content::shipped_with(REFUSED_FILE, REFUSED_DECLARATION)?;
    let whole = everything_written_for(&content::hud_refusal_over(root.path())?);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&declaration_path(REFUSED_FILE)),
            said.contains(REFUSED_ELEMENT),
            said.contains(REFUSED_FIELD),
            said.as_str(),
        ),
        (true, true, true, whole.as_str()),
        "a declaration the model refuses stops the client starting, and what stops it has to say \
         which file, which element and which field — a client that started here would show a \
         player a world with no crosshair and tell nobody why, and one that refused without \
         saying which of the three sends the author reading the file by hand"
    );
    Ok(())
}

#[test]
fn a_hud_file_that_is_not_toml_is_refused_naming_the_file_and_the_reason() -> TestResult {
    let root = content::shipped_with(REFUSED_FILE, NOT_TOML)?;
    let refused = content::hud_refusal_over(root.path())?;
    let reason = parser_reason_in(&refused)?;
    let whole = everything_written_for(&refused);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&declaration_path(REFUSED_FILE)),
            said.contains(&reason),
            said.contains(ELEMENT_CLAUSE),
            said.contains(FIELD_CLAUSE),
            said.as_str(),
        ),
        (true, true, false, false, whole.as_str()),
        "the reason comes from the parser rather than from this file: a test spelling a \
         diagnostic out by hand would be asserting the parser's wording rather than that any of \
         it reached the author. A file that is not TOML has no field to read a name out of, so \
         the whole file is what is named and no element and no field are"
    );
    Ok(())
}

#[test]
fn a_content_root_declaring_no_hud_is_read_without_a_word_about_the_hud() -> TestResult {
    let root = content::shipped_copy()?.declaring_no_hud()?;

    let ending = match mc_client::startup::prepare_scene(root.path()) {
        Ok(_) => Ending::Closed,
        Err(refused) => Ending::failed(&refused, ""),
    };

    assert_eq!(
        support::reported(&ending)?,
        "",
        "a root declaring no HUD is a valid root and not an empty one that went wrong: a client \
         refusing it would refuse every content root that has no interface to draw yet. What is \
         compared is the whole of what was written, so a refusal about anything at all shows up \
         here as the text it would have put on an author's terminal"
    );
    Ok(())
}

/// The whole of what the client writes for a run the HUD declarations refused,
/// derived from the refusal the loader itself produced rather than restated here.
fn everything_written_for(refused: &HudLoadError) -> String {
    format!("mycraft: {HUD_REFUSAL}: {refused}\n")
}

/// How a declaration file is written into a refusal — as a path, so the needle is
/// spelled the way this platform spells one.
fn declaration_path(file_name: &str) -> String {
    Path::new(content::HUD_DIRECTORY)
        .join(file_name)
        .display()
        .to_string()
}

/// The reason the parser gave, read out of `refused` rather than spelled here.
///
/// # Errors
///
/// Returns an error if the refusal is not about a declaration the parser could
/// not read, or if it carries no reason — in either case the scenario has no
/// parser's reason to look for, and searching for an empty one would find it
/// everywhere.
fn parser_reason_in(refused: &HudLoadError) -> Result<String, Box<dyn Error>> {
    let HudLoadError::Source(HudElementSourceError::Malformed(fault)) = refused else {
        return Err(format!(
            "this scenario needs a declaration the parser could not read, and what was refused \
             was: {refused}"
        )
        .into());
    };
    if fault.cause.is_empty() {
        return Err("the parser refused the declaration and gave no reason for it".into());
    }
    Ok(fault.cause.clone())
}
