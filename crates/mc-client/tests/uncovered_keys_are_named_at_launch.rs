//! The line a mod author reads when a key they declared has no baked art, over
//! real content rather than over two sets a test wrote.
//!
//! # Why not the sibling unit readings alone
//!
//! `src/notice_test.rs` asks the composer directly and is worth having, but both
//! of its arguments are lists this project wrote for it. The defect PRO-990
//! records is not about comparing two sets — it is that **nothing on a shipped
//! path ever built the two sets to compare**, so a notice printed on every launch
//! named nothing. What is asserted here is the pair of derivations that closes
//! that: the keys a registry read from a content root declares, and the keys the
//! built set beside that root decodes into texels.
//!
//! `base:water` was uncovered for three days and this notice — as it then was —
//! printed at every launch and said nothing about it. The first reading below is
//! that launch.
//!
//! # What this file still cannot see
//!
//! `changed_blocks_named_on_the_error_stream.rs`'s limitation, unchanged and for
//! the same reason: every reading here reaches the composer, and a client that
//! composes the line and never calls `say_stand_ins` leaves both green. That is
//! *policy is not wiring*, and the instrument for it is a real process.
//!
//! # The words are written out, never assembled from the client's own constants
//!
//! `notice_test.rs`'s rule. What a mod author reads is the artefact, and a test
//! that built the sentence from the same pieces the client does would agree with
//! it about a rewording neither had noticed.

mod support;

use mc_client::launch::declared_keys;
use mc_client::notice::stand_ins;

use support::{PreparedScene, TestResult, built_sets, prepare_scene_at};

/// A key no manifest bakes and no shipped block declares, and the block that
/// declares it.
///
/// A whole declaration rather than an edit of a shipped one, on
/// `an_unauthored_key_draws_a_generated_texture.rs`'s reasoning: this is about a
/// block somebody added, and a shipped block with its key changed would be a root
/// that had lost a texture rather than one that had gained a block.
const UNDRAWN_FILE: &str = "undrawn.luau";
const UNDRAWN_DECLARATION: &str = "return {\n\tname = \"example:undrawn\",\n\ttexture = \"example:undrawn\",\n\tsolid = true,\n}\n";

/// What that root's launch says, and the whole of it.
///
/// The singular clause, because this is a mod author's first block — which is the
/// case the notice exists for.
const THE_ONE_UNDRAWN_KEY: &str = "mycraft: `example:undrawn` draws a generated stand-in because \
                                   nothing has baked it, and that is not a failure";

#[test]
fn a_root_declaring_a_key_nothing_baked_names_that_key_and_no_other() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?
        .declaring_block(UNDRAWN_FILE, UNDRAWN_DECLARATION)?;

    let prepared = prepare_scene_at(root.path())?;

    assert_eq!(
        line_of(&prepared).as_deref(),
        Some(THE_ONE_UNDRAWN_KEY),
        "this is the whole of what a mod author's first block gets told: the key they wrote, and \
         that the launch is fine. The shipped keys beside it all have art and none of them is \
         named — a composer handed the declared keys and nothing else reports every block in the \
         root, which is the sentence this replaces one step less useless"
    );
    Ok(())
}

#[test]
fn the_shipped_root_covers_every_key_it_declares_and_nothing_is_said() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;

    let prepared = prepare_scene_at(root.path())?;

    assert_eq!(
        line_of(&prepared),
        None,
        "every key the shipped blocks declare is baked, so an ordinary launch of this game says \
         nothing about its art at all. A line here is one on every player's terminal on every run \
         — the defect PRO-990 records, which is what made the notice unreadable when `base:water` \
         really was uncovered"
    );
    Ok(())
}

/// What `prepared`'s launch says about the keys nothing baked for it.
///
/// Both halves off the one preparation, through the two calls the preparation
/// worker itself makes: the layer assignment states which keys the serving
/// content names and the texels are the decode of that root's built set, so there
/// is no second reader here for the two to disagree with.
fn line_of(prepared: &PreparedScene) -> Option<String> {
    stand_ins(
        &declared_keys(&prepared.resolution),
        &prepared.texels.keys(),
    )
}
