//! What the client does about a save whose blocks are no longer what they were:
//! the argument a player passes, and what they are told when passing it turned
//! them away.
//!
//! # Loading is the default, and the strictness is what has to be asked for
//!
//! A save whose blocks merely *behave* differently is loadable data. Refusing it
//! turned a content update into a world nobody could open, while the live reload
//! of the same edit accepted it and moved the player where that was genuinely
//! dangerous — the two answers were inverted relative to risk. So a launch with
//! nothing on its command line opens the world, and
//! `--refuse-changed-blocks` is how somebody asks for the old answer.
//!
//! # The refusal message is the whole user interface the strict answer has
//!
//! There is no dialog and no HUD in this build, so a player who asked for
//! strictness and was turned away sees one line of text and nothing else. If it
//! does not name **every** block that changed, they cannot tell which mod to put
//! back; if it does not say what to stop typing, they cannot get in. Either
//! omission is a dead end no amount of re-reading gets them out of.
//!
//! # The exact spelling and one letter short are each other's controls
//!
//! `acceptance_from` is an `any()` over one string, so the likeliest defect in a
//! rename is the constant moving while its value stays behind — or the value
//! moving while the sense of the comparison does not. The exact argument must
//! refuse; one letter short must load *and* still report, because an argument
//! nothing recognises is not an argument at all.

#[path = "support/changed_blocks.rs"]
mod changed_blocks;
#[path = "support/persistence.rs"]
mod persistence;

use changed_blocks::{
    ALPHA, NO_ARGUMENT, OMEGA, RECORDED_PLAYER, REFUSE_CHANGED_BLOCKS, REFUSING,
    REFUSING_MISSPELLED, RETEXTURED, STEADY, a_save_whose_blocks_are_all_unchanged,
    a_save_whose_two_blocks_were_redeclared, launch, reported_changed,
};
use persistence::{EVERY_DECLARED_CELL, TestResult, against, facing, refusal, save_in, stood_at};

/// What the refusal has to tell a player who asked for strictness and got it:
/// the argument, and that dropping it is the way in.
///
/// The verb is asserted because the advice is now the reverse of what it was. A
/// message that named the argument while still telling them to *pass* it would
/// contain the flag, satisfy a substring check for it, and send them round in a
/// circle.
const DROP_IT: &str = "drop `--refuse-changed-blocks` to load it anyway";

#[test]
fn a_save_whose_blocks_were_redeclared_is_played_with_nothing_on_the_command_line() -> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &NO_ARGUMENT)?;

    assert_eq!(
        (
            against(&launched, &save.written),
            stood_at(&launched),
            facing(&launched)
        ),
        (
            Ok((EVERY_DECLARED_CELL, Vec::new())),
            Ok(RECORDED_PLAYER.position.map(f32::to_bits)),
            Ok((
                RECORDED_PLAYER.yaw.to_bits(),
                RECORDED_PLAYER.pitch.to_bits()
            ))
        ),
        "two of this save's blocks behave differently now and a player who typed nothing gets \
         their world: the cells the save holds, and the position, yaw and pitch it recorded them \
         at. Anything less is a client that refused, or one that generated a world over a save \
         sitting right there. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn a_save_whose_blocks_were_redeclared_is_refused_when_the_player_asked_for_strictness()
-> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let told = refusal(&launch(&save, &path, &REFUSING)?);

    assert_eq!(
        (
            told.contains(ALPHA),
            told.contains(OMEGA),
            told.contains(STEADY),
            told.contains(RETEXTURED)
        ),
        (true, true, false, false),
        "somebody who passed `{REFUSE_CHANGED_BLOCKS}` asked to be turned away rather than shown a \
         world whose blocks have moved, and the line they get has to name every block that is no \
         longer what it was — {ALPHA} and {OMEGA}, both of them, not the first one it met. \
         {STEADY} did not change and {RETEXTURED} only looks different; naming either would send \
         them looking for a mod that is fine. They were told: {told}"
    );
    Ok(())
}

#[test]
fn the_refusal_names_the_argument_that_caused_it_as_the_thing_to_drop() -> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let told = refusal(&launch(&save, &path, &REFUSING)?);

    assert!(
        told.contains(DROP_IT),
        "this refusal exists only because the player asked for it, so the way out of it is to stop \
         asking. The line has to say so in those terms — `{DROP_IT}` — because a message that \
         named the argument and told them to pass it would send them round in a circle, and a \
         message that named no argument at all leaves somebody who has forgotten what they typed \
         with nowhere to go. They were told: {told}"
    );
    Ok(())
}

#[test]
fn a_save_whose_blocks_all_still_match_is_played_even_when_strictness_was_asked_for() -> TestResult
{
    let save = a_save_whose_blocks_are_all_unchanged()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &REFUSING)?;

    assert_eq!(
        (against(&launched, &save.written), stood_at(&launched)),
        (
            Ok((EVERY_DECLARED_CELL, Vec::new())),
            Ok(RECORDED_PLAYER.position.map(f32::to_bits))
        ),
        "`{REFUSE_CHANGED_BLOCKS}` asks for a save whose blocks have *changed* to be refused, and \
         nothing about this save's blocks has changed. A client that read the argument as \
         \"refuse\" rather than as \"refuse if\" turns a player away from a world nothing is wrong \
         with, and it is turned away here. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}

#[test]
fn an_argument_one_letter_short_of_the_real_one_is_not_the_real_one() -> TestResult {
    let save = a_save_whose_two_blocks_were_redeclared()?;
    let path = save_in(&save.directory);

    let launched = launch(&save, &path, &REFUSING_MISSPELLED)?;

    assert_eq!(
        (
            against(&launched, &save.written),
            reported_changed(&launched)
        ),
        (
            Ok((EVERY_DECLARED_CELL, Vec::new())),
            Ok(vec![ALPHA.to_owned(), OMEGA.to_owned()])
        ),
        "the parse is an equality over one spelling, so the likeliest defect in renaming it is the \
         constant moving while its value stays where it was — or a prefix comparison creeping in. \
         `--refuse-changed-block` is not the argument, so this launch is the default launch: the \
         world opens and the changed blocks are still reported. It answered: {}",
        refusal(&launched)
    );
    Ok(())
}
