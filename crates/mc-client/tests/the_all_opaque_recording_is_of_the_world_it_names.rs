//! What the committed all-opaque recording is, judged without judging the
//! oracle that produced it.
//!
//! # What these readings are, and what they deliberately are not
//!
//! `crates/mc-client/tests/fixtures/all_opaque/predictions.txt` holds 1 728
//! classifications taken from this tree before a block could declare an opacity.
//! The comparison those values exist for — *this tree's oracle predicts exactly
//! them* — **is now the first reading below**, and it could not have been
//! written when the file was: at that commit it would have asserted that a
//! recording of this tree matched this tree, which is true by construction and
//! reports nothing. The oracle has since gained a second rule, and what the
//! recording says is that the rule arrived without moving one prediction over a
//! world holding nothing that passes light.
//!
//! The rest judge the **fixture** rather than a prediction: that it
//! covers the grid it claims to, that it is written in the one format it is read
//! in, that the classes in it are the classes the world can offer, that the root
//! it was taken over generates the world it says it does, and that no
//! declaration under that root has acquired the field whose absence is the whole
//! premise. A recording nothing reads for a whole phase is a recording that rots
//! in silence — a hand-maintained list nobody compares whole is how two mirrors
//! of a nine-name list sat at six for a phase apiece.
//!
//! **Every one of them passes on the tree that produced the file**, which is
//! what a fixture guard does. They are here to redden on the tree that damages
//! it.

mod support;

use mc_sim::replay::ReplayWorld;

use support::all_opaque::{self, THE_BLOCKS, THE_DECLARATION_FILES, THE_FORBIDDEN_FIELD};
use support::oracle::{self, SAMPLE_COUNT};
use support::{TestResult, content_registry};

/// What a viewpoint whose sample grid is the declared one reports.
const THE_WHOLE_GRID: &str = "every declared sample pixel, in order";

/// Everything a class in the recording may be: the four blocks the fixture root
/// declares, and the sky.
///
/// Assembled from the fixture module's own list rather than discovered from the
/// recording, which is the thing under test — a set read out of the file would
/// agree with it whatever it came to hold.
fn the_classes() -> Vec<String> {
    let mut classes: Vec<String> = THE_BLOCKS.iter().map(|&block| block.to_owned()).collect();
    classes.push(oracle::SKY.to_owned());
    classes.sort();
    classes
}

/// The oracle's second rule arrived without moving a single prediction over a
/// world in which every drawn block stops all the light.
///
/// **The one comparison the committed recording exists for.** Its expected
/// values were taken on a tree where `Opacity` did not exist at all, by a judge
/// that had no second rule to get wrong, and committed before this spec's first
/// implementation commit — so they are a reading of a *different program* rather
/// than a number snapshotted from a run of the code under test. That is the
/// distinction `testing.md` §2 turns on, and it is why nothing regenerates the
/// file: re-recording it against a tree that already carries the change would
/// destroy exactly the property it exists to carry.
///
/// **Compared whole and in order**, so a moved sample, a dropped viewpoint, a
/// class that changed and a grid that stopped summing to itself are four
/// distinct failures of one comparison rather than one count that saw none of
/// them.
#[test]
fn the_oracles_second_rule_moves_no_prediction_over_a_world_that_stops_all_the_light() -> TestResult
{
    assert_eq!(
        all_opaque::predicted()?,
        all_opaque::recorded()?,
        "the second rule only ever fires for a block declaring a degree below one, and no          declaration under this fixture's root states a degree at all — the reading below is what          keeps that true. So every one of these {} classifications has to come back exactly as it          was recorded on the pre-change tree. A difference here is the second rule reaching a ray          that crosses nothing translucent, which would move every frame this suite grades and          would be invisible to a suite whose expectations all came from the same tree",
        SAMPLE_COUNT * all_opaque::VIEWPOINTS.len()
    );
    Ok(())
}

/// One viewpoint of the recording, reduced to what it claims to be about.
#[derive(Debug, PartialEq, Eq)]
struct Covered {
    viewpoint: String,
    eye: [String; 3],
    target: [String; 3],
    /// [`THE_WHOLE_GRID`], or the first place the recorded pixels part from the
    /// declared ones.
    grid: String,
}

/// The recording covers exactly the declared viewpoints, each carrying the
/// camera it names and the whole declared sample grid in order.
///
/// **An enumerated verdict rather than a count.** A recording truncated to a
/// hundred samples, one with a viewpoint dropped, one whose eye was edited to a
/// camera the values were never marched from, and one whose lines were sorted
/// into a different order are four distinct failures of this one comparison; a
/// check on the number of lines would see only the first.
#[test]
fn the_recording_covers_every_declared_sample_of_every_declared_viewpoint() -> TestResult {
    let covered: Vec<Covered> = all_opaque::recorded()?
        .into_iter()
        .map(|recorded| Covered {
            viewpoint: recorded.viewpoint,
            eye: recorded.eye,
            target: recorded.target,
            grid: grid_of(&recorded.samples),
        })
        .collect();

    assert_eq!(
        covered,
        all_opaque::VIEWPOINTS
            .iter()
            .map(|viewpoint| Covered {
                viewpoint: viewpoint.name.to_owned(),
                eye: all_opaque::written(viewpoint.eye),
                target: all_opaque::written(viewpoint.target),
                grid: THE_WHOLE_GRID.to_owned(),
            })
            .collect::<Vec<_>>(),
        "the recording is read sample by sample against a march from these same three cameras, \
         so a grid it does not cover is a comparison that would quietly be about fewer samples \
         than it claims, and an eye it does not carry is one the values were never taken from"
    );
    Ok(())
}

/// The recording is written in the one format it is read in.
///
/// Reading it and writing it back has to give the file back line for line, once
/// the header and the blank lines the format ignores are set aside.
///
/// **What only this can see is a line the parser absorbs without it ever
/// reaching a comparison**, which is measured rather than argued: a duplicated
/// `eye` line reddens this and leaves the other five green, because the second
/// one overwrites the first and the recording that comes out is the recording
/// that went in. Measured the other way too — a deleted *sample* line does
/// **not** redden this one, since the shortened file renders back to itself; the
/// coverage reading above is that one's only witness. The two are a pair and
/// neither subsumes the other.
#[test]
fn the_recording_reads_back_as_the_one_format_it_is_written_in() -> TestResult {
    let text = all_opaque::recorded_text()?;

    assert_eq!(
        all_opaque::rendered(&all_opaque::recorded()?),
        all_opaque::body_of(&text),
        "the writer that produced this file and the reader that will grade it against a fresh \
         march have to agree about every line of it. A file only the reader accepts is one whose \
         provenance stops meaning anything the moment somebody edits it"
    );
    Ok(())
}

/// Every class the recording names is one the all-opaque world can offer, and
/// every class that world can offer is named.
///
/// **Both directions in one comparison, which is the point.** A recording that
/// had come to hold nothing but the sky satisfies "no class outside the list"
/// perfectly, and a recording naming a block the root does not declare satisfies
/// "every class appears". Comparing the whole set rejects both.
#[test]
fn the_recording_names_exactly_the_classes_the_all_opaque_world_can_offer() -> TestResult {
    let mut named: Vec<String> = all_opaque::recorded()?
        .into_iter()
        .flat_map(|recorded| recorded.samples.into_iter().map(|(_, class)| class))
        .collect();
    named.sort();
    named.dedup();

    assert_eq!(
        named,
        the_classes(),
        "the three viewpoints were chosen so that between them they name every class this world \
         holds — one apiece for the three the world is sparse in. A class missing from this set \
         is a march that has stopped answering about one of them, and a class outside it is a \
         march answering about a world this root does not describe"
    );
    Ok(())
}

/// The fixture root generates the world the shipped root generates.
///
/// This is the premise that makes the recording worth anything: the values
/// describe the replay's own voxels rather than some private world of the
/// suite's. It holds because `ReplayWorld::generate` takes its shape from the
/// seed and reads only names out of the registry it is handed, and it keeps
/// holding through phase 3 — declaring an opacity on the shipped sea moves no
/// voxel.
#[test]
fn the_all_opaque_root_generates_the_world_the_shipped_root_generates() -> TestResult {
    let fixture = all_opaque::registry()?;
    let shipped = content_registry()?;

    assert!(
        all_opaque::world(&fixture)? == ReplayWorld::generate(mc_sim::REPLAY_SEED, &shipped)?,
        "the recording is offered as a reading of the replay world, and the root it was taken \
         over is not the one the game ships. What makes the two worlds one world is that both \
         roots declare the same four names — measured, and not the ids they are registered \
         under, which may differ without moving a voxel. A difference here means the fixture \
         root has drifted and the recording is about something else"
    );
    Ok(())
}

/// No declaration under the fixture root states an opacity.
///
/// The recording is a recording of a world in which **every** drawn block is
/// opaque. A declaration here that stated an opacity would not fail the
/// comparison the recording feeds — it would quietly change what that comparison
/// is about, which is worse. Reported per file rather than as a single yes or
/// no, so a file that has gone missing shortens the list instead of reading as a
/// file that states nothing.
#[test]
fn no_block_the_all_opaque_root_declares_states_an_opacity() -> TestResult {
    assert_eq!(
        all_opaque::declarations_stating_an_opacity()?,
        THE_DECLARATION_FILES
            .iter()
            .map(|file_name| ((*file_name).to_owned(), false))
            .collect::<Vec<_>>(),
        "this root exists to state no `{THE_FORBIDDEN_FIELD}` at any block, on either side of \
         SPEC-031: the loader that cannot read the field ignores it and the loader that can reads \
         its absence as 1.0. A block here that declared one would leave the recording a reading \
         of a world nothing else describes"
    );
    Ok(())
}

/// That same reading reports a declaration that does state one.
///
/// The positive control the reading above cannot do without. An absence
/// assertion goes green forever the day it stops being able to look, and a scan
/// that had come to answer "no" to every declaration — a changed field name, a
/// comment stripper that ate the whole file — is indistinguishable from a clean
/// root without this.
#[test]
fn the_same_reading_reports_a_declaration_that_does_state_an_opacity() -> TestResult {
    let doctored = "-- opacity is named in this comment and does not count\n\
                    return {\n\
                    \tname = \"base:water\",\n\
                    \topacity = 0.5,\n\
                    }\n";

    assert!(
        all_opaque::states_an_opacity(doctored),
        "the reading beside this one asserts an absence over four files, and an absence is what a \
         scan reports when it has stopped being able to look. This is the fixture that does state \
         the field, and it has to be reported — in the declaration and not in the comment above \
         it, which is why the comment names it too"
    );
    Ok(())
}

/// Where the recorded pixels part from the declared grid, or
/// [`THE_WHOLE_GRID`] where they do not.
fn grid_of(samples: &[((u32, u32), String)]) -> String {
    let declared = oracle::sample_pixels();
    if let Some((at, (recorded, expected))) = samples
        .iter()
        .map(|(pixel, _)| *pixel)
        .zip(declared.iter().copied())
        .enumerate()
        .find(|(_, (recorded, expected))| recorded != expected)
    {
        return format!(
            "{} of {SAMPLE_COUNT} samples, first parting from the declared grid at index {at}: \
             recorded {recorded:?} where the grid declares {expected:?}",
            samples.len()
        );
    }
    if samples.len() == declared.len() {
        return THE_WHOLE_GRID.to_owned();
    }
    format!(
        "{} of {SAMPLE_COUNT} samples, agreeing with the declared grid as far as they go",
        samples.len()
    )
}
