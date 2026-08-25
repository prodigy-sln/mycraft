//! What a mod author reads when the number they just typed is one the engine
//! cannot use, and what the game goes on serving while they fix it.
//!
//! # These are refusals a launch already produces, reached through a reload
//!
//! A resistance below zero and a resistance that is not a finite number are
//! refused by `mc_sim::content::load` today, naming the file, the block and the
//! field, and a reload's build stage is that same call on a worker. So these are
//! **controls on the path** rather than tests of new vocabulary: they redden
//! exactly if the reload comes to reach content through something else, and a new
//! fault type written to satisfy one of them is the signal that it has.
//!
//! `reload_refuses_a_broken_declaration.rs` holds the same shape for a chunk that
//! will not compile, a misspelled field and two files claiming one name. What is
//! narrower here is that `move_resistance` is the first field whose value can be
//! *well formed and still refused* — every other refusal on that path is about the
//! shape of a declaration, and this one is about a number inside a shape that is
//! perfectly good. A loader that read the field with the wrong kind of check, or
//! that clamped instead of refusing, produces a root that loads, and the reload
//! would then take up content nobody declared.
//!
//! # No refusal's wording is spelled here
//!
//! Each expectation is asked of a second read of the same root, which reaches the
//! failure without going near the reload, and the reported text has to **end** in
//! it — so whatever framing a reload puts above a refusal, the sentence naming the
//! file survives to the person. A reworded refusal moves both sides together; a
//! *dropped* cause moves only the reported side, which is the asymmetry a
//! snapshotted string does not have.
//!
//! What each scenario adds on top is the words it requires by name — the file, the
//! block and `move_resistance` — because a comparison against the whole chain would
//! go on agreeing if the loader quietly stopped filling one of them in. A number
//! refused without the field named leaves an author reading their file line by line
//! for the one the engine would not take.
//!
//! # Both of them say what the game is still serving
//!
//! A refusal that lost the content would be a worse outcome than the bad number, so
//! each assertion carries the four blocks the client is still serving beside the
//! refusal. That half is what tells a refusal apart from a half-applied candidate —
//! and it is the half the scenario names, because an author fixing a typo must not
//! find the world they were standing in gone.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use reload::{GRASS, WATER, WATER_FILE, shipped};
use reload_watch::{
    Refusal, a_client_on, block_path, declaration_named, naming, refusal, restating_raw, serving,
    the_four_shipped_blocks, the_loaders_own_words, until_settled,
};
use support::TestResult;

/// The field the two declarations below get wrong, and the two values they get
/// wrong in the two ways the loader distinguishes.
///
/// `-1` is a number the engine understands perfectly and may not use: a divisor of
/// `1 / (1 + r)` would be negative, which is a walk carried backwards. `0/0` is not
/// a number at all, and would turn every position the player holds into one.
const THE_FIELD: &str = "move_resistance";
const A_NEGATIVE_RESISTANCE: &str = "-1";
const A_RESISTANCE_THAT_IS_NOT_A_NUMBER: &str = "0/0";

#[test]
fn a_resistance_below_zero_leaves_the_content_serving_and_names_the_file_block_and_field()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating_raw(root, WATER_FILE, &water_resisting(A_NEGATIVE_RESISTANCE))?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(
                &crossed,
                &words,
                &naming(&[&declaration_named(WATER_FILE), WATER, THE_FIELD])
            ),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "`{A_NEGATIVE_RESISTANCE}` is a number the engine understands and cannot use, so it is \
         refused rather than clamped to zero — a clamp would register a volume the author never \
         declared and leave them looking for the minus sign in a file that loaded. All three of \
         the file, the block and the field have to reach them, and the four blocks beside the \
         refusal are what says a bad number cost them nothing but the save"
    );
    Ok(())
}

#[test]
fn a_resistance_that_is_not_a_finite_number_leaves_the_content_serving_and_names_the_field()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating_raw(
        root,
        WATER_FILE,
        &water_resisting(A_RESISTANCE_THAT_IS_NOT_A_NUMBER),
    )?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, WATER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(
                &crossed,
                &words,
                &naming(&[&declaration_named(WATER_FILE), WATER, THE_FIELD])
            ),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "`{A_RESISTANCE_THAT_IS_NOT_A_NUMBER}` is a Luau expression that evaluates without \
         complaint and produces something no division can use. It is the second of the two ways a \
         well-formed declaration can carry a number the engine refuses, and it is read on the same \
         instrument as the first so that the pair says the refusal is about the value rather than \
         about a sign the reader happened to notice"
    );
    Ok(())
}

/// What `base:water` says when an author has typed `resistance` where a resistance
/// belongs and left everything else exactly as the shipped declaration has it.
///
/// **The value is spliced in raw, not rendered from a Rust number**, because the
/// two values these scenarios use are things a Rust `f32` cannot carry to the file
/// intact: one of them is a Luau expression. Every other field is written out in
/// full so that the file the author saved differs from the one they had in the
/// number and in nothing else.
fn water_resisting(resistance: &str) -> String {
    [
        "return {".to_owned(),
        format!("\tname = \"{WATER}\","),
        format!("\ttexture = \"{WATER}\","),
        "\tsolid = false,".to_owned(),
        "\tbreakable = false,".to_owned(),
        "\treplaceable = true,".to_owned(),
        "\tdrawn = true,".to_owned(),
        "\toccludes = false,".to_owned(),
        "\ttargetable = true,".to_owned(),
        "\tswimmable = true,".to_owned(),
        format!("\t{THE_FIELD} = {resistance},"),
        "}".to_owned(),
        String::new(),
    ]
    .join("\n")
}
