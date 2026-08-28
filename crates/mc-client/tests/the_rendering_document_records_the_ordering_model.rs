//! What `docs/technical/rendering.md` owes about the chosen ordering model, and
//! whether the numbers it states are the ones the engine and the art give.
//!
//! # A documentation scenario without a guard is a scenario with no falsifier
//!
//! The requirement fires when the document is *read*, which means prose alone
//! satisfies it — and prose can be deleted tomorrow with nothing going red. So
//! the document is read by a test, and the verdict is a **total enumeration**
//! rather than an absence: `EverythingTheModelOwes` rejects every other answer
//! including "I could not look", which an emptiness check cannot.
//!
//! # The guard reads structure and values, and constrains wording as little as
//! it can
//!
//! **When a guard and a document disagree, the one shaped wrongly is usually the
//! guard, because the document has a reader and the guard does not.** A guard
//! dictating how a sentence may be written has stopped checking the document and
//! started editing it, and the pressure it creates runs toward prose shaped to
//! satisfy a matcher — an agreement test relocated into documentation, true of
//! neither the engine nor the world.
//!
//! So each thing owed is read by the least constraining mechanism available:
//!
//! - **the two depth-write settings come out of the document's own table**, a
//!   row per draw and a column per setting. Emphasis, rewording, sentence order
//!   and prose style are then all free, and the guard still fails if the page
//!   stops stating the setting — the only thing it was ever for. A table cell
//!   cannot be bolded into invisibility;
//! - **the figures are looked for as values**, so the page may say them however
//!   it likes and cannot say them wrongly;
//! - **only the model's name and the pass order are matched as phrases**, with
//!   emphasis normalised out of both sides, because there is no structure there
//!   to read.
//!
//! # The numeric half carries the weight
//!
//! Every number this reading looks for is compared against something
//! **measured** — the two compositions against the frames the fixture draws now,
//! the camera against the fixture's own declared pose, the spread and the
//! ceiling against the shipped art and the crossings the declared captures hold,
//! and the tolerance's total against the sum of its own terms. Three of them are
//! three kinds of claim and are checked three ways:
//!
//! - the **spread** is a property of an image on disk, so it is an equality at
//!   the precision the page states it to;
//! - the **ordering residual** is a derived *bound*, and an equality against it
//!   is red on a correct engine: a quarter of the spread is 0.7904 and the page
//!   rounds it to 0.79, so `measured ≤ stated` fails by four ten-thousandths and
//!   the cheapest green would be editing the bound down. What is checked is the
//!   identity the page itself asserts — the residual is a quarter of the spread
//!   it also states;
//! - the **floor** is a total with its terms beside it, so the check is the
//!   arithmetic: a term that moved while the total stayed reddens on the sum.

mod support;

use std::error::Error;
use std::fs;

use mc_core::id::BlockName;
use mc_testkit::frame::{Thresholds, compare, read_png};

use support::artefact::{Emitted, composed_when, frame_named_on_the_page, frame_on_the_page, shot};
use support::composite::{Palette, nearest_between};
use support::frames::CAPTURE_SIZE;
use support::goldens::DECLARED_TICKS;
use support::oracle::{Voxels, crossed_samples};
use support::translucency::{EYE, LOOK_AT, TELLS_THEM_APART};
use support::{PreparedScene, TestResult, prepare_scene, repository_root};

/// What the document owes, item by item.
///
/// **An enumerated verdict and never an emptiness.** A reading that answered
/// "nothing missing" because it had stopped being able to read the page would be
/// indistinguishable from one reading a complete page; this answers what it
/// found instead.
#[derive(Debug, PartialEq, Eq)]
enum WhatTheModelOwes {
    EverythingTheModelOwes,
    Missing(Vec<&'static str>),
}

impl WhatTheModelOwes {
    /// The verdict `missing` amounts to.
    fn of(missing: Vec<&'static str>) -> Self {
        if missing.is_empty() {
            Self::EverythingTheModelOwes
        } else {
            Self::Missing(missing)
        }
    }
}

/// The two claims with no structure to read them out of, each with the phrase it
/// is looked for by.
///
/// **Kept to two on purpose.** Emphasis is normalised out of both sides so a
/// bolded word cannot hide a claim, but a phrase match still constrains wording,
/// and everything with a table or a number behind it is read that way instead.
const AS_PHRASES: [(&str, &str); 2] = [
    (
        "the ordering model, named",
        "one unsorted blended pass, depth-test on, depth-write off",
    ),
    (
        "the order the two draws run in",
        "One render pass, two draws, opaque first",
    ),
];

/// What the table's depth-write column has to say about each draw, in row order.
const DEPTH_WRITES: [(&str, &str); 2] = [
    ("the opaque draw writing depth", "on"),
    ("the blended draw not writing depth", "off"),
];

/// What a page missing a picture it names reports.
const NO_COMMITTED_FRAME: &str = "a committed frame at the path the page names";

#[test]
fn the_rendering_document_names_the_model_the_passes_the_camera_and_both_frames() -> TestResult {
    let page = rendering_document()?;

    assert_eq!(
        WhatTheModelOwes::of(structurally_missing_from(&page)?),
        WhatTheModelOwes::EverythingTheModelOwes,
        "the chosen ordering model is wrong where two translucent surfaces of different colours \
         overlap, and the spec makes the model's stated behaviour a deliverable rather than \
         something a later reader discovers. So the page has to name the model, say which of the \
         two draws runs first, state in its own table what each does about depth, give the camera \
         the artefact is shown from — {EYE:?} looking at {LOOK_AT:?} — and carry the two frames, \
         which are committed beside it and are the difference the artefact consists of. A missing \
         line here is a document describing the engine less completely than the engine behaves"
    );
    Ok(())
}

#[test]
fn a_document_short_of_any_one_of_those_reports_that_one() -> TestResult {
    let mut reported = Vec::new();
    for (_, short) in a_page_short_of_each() {
        reported.push(structurally_missing_from(&short)?);
    }

    assert_eq!(
        reported,
        a_page_short_of_each()
            .into_iter()
            .map(|(what, _)| vec![what])
            .collect::<Vec<_>>(),
        "a scan asserting only an absence goes green forever the day it stops being able to look, \
         so the same reading is driven over a page carrying every one of these and then over that \
         page with each taken away in turn — a phrase deleted, a table cell emptied, a table cell \
         saying the wrong thing, a coordinate gone, an image no longer named. Each has to be \
         reported, and reported alone: a reading answering the whole list, or the wrong member of \
         it, is one whose verdict nobody could act on"
    );
    Ok(())
}

#[test]
fn the_colours_the_rendering_document_states_are_the_colours_the_model_draws() -> TestResult {
    let stated = triples_in(&rendering_document()?);

    let mut drawn = Vec::new();
    for order in Emitted::BOTH {
        let Some(shown) = shot(order)? else {
            return Ok(());
        };
        let committed = read_png(&frame_on_the_page(order)?)?;
        drawn.push((
            order.described(),
            nearest_between(&stated, &[composed_when(order)])? <= TELLS_THEM_APART,
            compare(&committed, &shown.frame, &Thresholds::default()).failing_pixels,
        ));
    }

    assert_eq!(
        drawn,
        Emitted::BOTH
            .map(|order| (order.described(), true, 0))
            .to_vec(),
        "the page states the colour each emission order composites to, and those colours are \
         checked against the frames this fixture draws now rather than against a constant carried \
         here — a document and a test written to agree with each other would be true of neither \
         the engine nor the world. The second element is that colour, read out of the page as a \
         value so the page may write it however it likes; the third is the committed frame beside \
         the page, which has to be the frame the fixture still draws, or the picture a reader is \
         shown is of an engine that has moved on. Composed now: {:?} and {:?}",
        composed_when(Emitted::FartherFirst),
        composed_when(Emitted::NearerFirst)
    );
    Ok(())
}

#[test]
fn the_figures_the_rendering_document_states_are_the_ones_the_art_and_the_captures_give()
-> TestResult {
    let page = normalised(&rendering_document()?);
    let prepared = prepare_scene()?;
    let (spread, ceiling) = (
        support::art::spread_of("base:water", &prepared.texels)?,
        the_measured_ceiling(&prepared)?,
    );
    let floor = numbers_after(&page, "The floor is", 4);

    assert_eq!(
        what_the_page_states(&page, ceiling, &floor),
        (
            vec![to_two_places(spread)],
            vec![to_two_places(spread / 4.0)],
            true,
            4,
            true,
        ),
        "every figure is checked against what the art and the declared captures give, at the \
         precision the page states it to. The spread is a property of one image, so it is an \
         equality. The residual is a *bound*: an equality would be red on a correct engine — a \
         quarter of the spread is {:.4} and the page rounds it — so what is checked is the \
         identity the page asserts. The third element is the ceiling, {ceiling:.2}, the nearest a \
         composition stands from a colour one of its own operands draws unblended; it is looked \
         for as a value anywhere on the page, so the page may say it however it likes and cannot \
         say it wrongly. The last is the floor's own arithmetic, which reddens when a term moves \
         and the total is left behind",
        spread / 4.0
    );
    Ok(())
}

/// What the page states about the spread, the residual, the ceiling and the
/// floor, in the form the reading above compares.
type Figures = (Vec<i64>, Vec<i64>, bool, usize, bool);

fn what_the_page_states(page: &str, ceiling: f64, floor: &[f64]) -> Figures {
    (
        in_hundredths(numbers_after(page, "texel spread", 1)),
        in_hundredths(numbers_after(page, "ordering residual \u{2264}", 1)),
        in_hundredths(every_number_in(page)).contains(&to_two_places(ceiling)),
        floor.len(),
        sums_to(floor),
    )
}

/// Everything `page` does not carry, in a fixed order so a verdict reads the
/// same way twice.
///
/// The two frames have to be **named by the page and present on disk**: a page
/// pointing at a picture nobody committed shows a reader nothing.
fn structurally_missing_from(page: &str) -> Result<Vec<&'static str>, Box<dyn Error>> {
    let flattened = normalised(page);
    let mut missing: Vec<&'static str> = AS_PHRASES
        .into_iter()
        .filter(|(_, phrase)| !flattened.contains(phrase))
        .map(|(what, _)| what)
        .collect();
    let stated = depth_writes_in(page);
    for (at, (what, setting)) in DEPTH_WRITES.into_iter().enumerate() {
        if stated.get(at).map(String::as_str) != Some(setting) {
            missing.push(what);
        }
    }
    for (what, at) in [
        ("the camera the artefact is shown from", EYE),
        ("what that camera looks at", LOOK_AT),
    ] {
        if !flattened.contains(&written(at)) {
            missing.push(what);
        }
    }
    for (order, what) in Emitted::BOTH.into_iter().zip(THE_FRAMES) {
        if !flattened.contains(frame_named_on_the_page(order)) {
            missing.push(what);
        }
        if !frame_on_the_page(order)?.is_file() {
            missing.push(NO_COMMITTED_FRAME);
        }
    }
    Ok(missing)
}

/// What each frame is called in a verdict, in [`Emitted::BOTH`] order.
const THE_FRAMES: [&str; 2] = [
    "the frame the farther pane emitted first draws",
    "the frame the nearer pane emitted first draws",
];

/// What the page's own table says about each draw's depth write, in row order.
///
/// **Structure read rather than wording matched.** The header row is found by
/// the columns it names, and the rows after it up to the first line that is not
/// a row are the draws. A cell's emphasis is taken off, so a document stays free
/// to stress the word its sentence turns on.
fn depth_writes_in(page: &str) -> Vec<String> {
    let mut lines = page.lines().skip_while(|line| !names_the_columns(line));
    lines.next();
    lines
        .skip_while(|line| line.replace([' ', '|'], "").chars().all(|mark| mark == '-'))
        .take_while(|line| line.trim_start().starts_with('|'))
        .filter_map(|row| {
            row.trim()
                .trim_matches('|')
                .split('|')
                .next_back()
                .map(|cell| cell.replace(['*', '`'], "").trim().to_owned())
        })
        .collect()
}

/// Whether `line` is the header row of the table the two draws are stated in.
fn names_the_columns(line: &str) -> bool {
    ["pipeline", "blend", "depth test", "depth write"]
        .iter()
        .all(|column| line.contains(column))
}

/// A page carrying every one of them, and that same page with each taken away.
fn a_page_short_of_each() -> Vec<(&'static str, String)> {
    let whole = a_page_carrying_everything();
    let mut short: Vec<(&'static str, String)> = AS_PHRASES
        .into_iter()
        .map(|(what, phrase)| (what, whole.replacen(phrase, "", 1)))
        .collect();
    short.push((
        DEPTH_WRITES[0].0,
        whole.replacen("| Less | on |", "| Less |  |", 1),
    ));
    short.push((
        DEPTH_WRITES[1].0,
        whole.replacen("| Less | off |", "| Less | on |", 1),
    ));
    short.push((
        "the camera the artefact is shown from",
        whole.replacen(&written(EYE), "", 1),
    ));
    short.push((
        "what that camera looks at",
        whole.replacen(&written(LOOK_AT), "", 1),
    ));
    for (order, what) in Emitted::BOTH.into_iter().zip(THE_FRAMES) {
        short.push((what, whole.replacen(frame_named_on_the_page(order), "", 1)));
    }
    short
}

/// A page carrying every one of them, which exists so each can be taken away.
///
/// **Written to satisfy the scan, and that is legitimate here and nowhere
/// else.** A control fixture's whole purpose is to be the thing the reading
/// should pass over; the document it stands in for is judged by the readings
/// above, which check values and structure rather than wording.
fn a_page_carrying_everything() -> String {
    format!(
        "{}\n{}\n| | pipeline | blend | depth test | depth write |\n|---|---|---|---|---|\n\
         | first draw | mycraft terrain | none | Less | on |\n\
         | second draw | mycraft terrain blended | SrcAlpha | Less | off |\n\
         The camera stands at {} looking at {}, and the frames are {} and {}.\n",
        AS_PHRASES[0].1,
        AS_PHRASES[1].1,
        written(EYE),
        written(LOOK_AT),
        frame_named_on_the_page(Emitted::FartherFirst),
        frame_named_on_the_page(Emitted::NearerFirst),
    )
}

/// `docs/technical/rendering.md` as it stands.
fn rendering_document() -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(
        repository_root()?
            .join("docs")
            .join("technical")
            .join("rendering.md"),
    )?)
}

/// `text` with the emphasis a reader sees and the line breaks a paragraph
/// carries taken out, so a sentence is looked for as a sentence.
fn normalised(text: &str) -> String {
    text.replace(['*', '`'], "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// A camera coordinate as a page states one.
fn written(at: [f32; 3]) -> String {
    format!("({}, {}, {})", at[0], at[1], at[2])
}

/// The nearest a composition of the sea stands from a colour one of its own
/// operands draws unblended, over every crossing the declared captures hold.
///
/// **The tolerance's real ceiling for a blended reading**, and not the distance
/// between two layers' means: what a composition can be mistaken for is one of
/// the two things it was composed from — the sea failing to draw, and the sea
/// drawing without blending, the second being the defect this whole spec exists
/// to fix. It needs no device, because a composition is arithmetic over what the
/// content declares and the art holds.
fn the_measured_ceiling(prepared: &PreparedScene) -> Result<f64, Box<dyn Error>> {
    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let palette = Palette::of(&prepared.registry, &prepared.resolution, &prepared.texels);
    let sea = BlockName::parse("base:water")?;
    let mut nearest = f64::MAX;
    for tick in DECLARED_TICKS {
        let camera =
            support::frames::player_pose(u32::from(tick), &prepared.world, &prepared.registry)?;
        let crossings = crossed_samples(&camera, CAPTURE_SIZE, &voxels)?;
        for (_, crossed) in crossings
            .iter()
            .filter(|(_, crossed)| crossed.layers.iter().any(|layer| layer.block == sea))
        {
            nearest = nearest.min(palette.unblended_stands_from(crossed)?);
        }
    }
    Ok(nearest)
}

/// Every three-number group `text` states, as a colour.
fn triples_in(text: &str) -> Vec<[u8; 3]> {
    let mut found = Vec::new();
    for group in text.split(['(', '[']).skip(1) {
        let Some(inside) = group.split([')', ']']).next() else {
            continue;
        };
        let parts: Vec<&str> = inside.split(',').collect();
        let numbers: Vec<u8> = parts
            .iter()
            .filter_map(|at| at.trim().parse().ok())
            .collect();
        if let ([red, green, blue], 3) = (&numbers[..], parts.len()) {
            found.push([*red, *green, *blue]);
        }
    }
    found
}

/// The first `how_many` numbers `text` states after `needle`.
fn numbers_after(text: &str, needle: &str, how_many: usize) -> Vec<f64> {
    let Some(at) = text.find(needle) else {
        return Vec::new();
    };
    let mut found = every_number_in(&text[at + needle.len()..]);
    found.truncate(how_many);
    found
}

/// Each of `stated` as a whole number of hundredths.
fn in_hundredths(stated: Vec<f64>) -> Vec<i64> {
    stated.into_iter().map(to_two_places).collect()
}

/// Every number `text` states, in order.
fn every_number_in(text: &str) -> Vec<f64> {
    let mut found = Vec::new();
    let mut current = String::new();
    for letter in text.chars().chain(" ".chars()) {
        if letter.is_ascii_digit() || (letter == '.' && !current.is_empty()) {
            current.push(letter);
            continue;
        }
        let parsed = current.trim_end_matches('.').parse::<f64>();
        current.clear();
        if let Ok(value) = parsed {
            found.push(value);
        }
    }
    found
}

/// Whether the terms `floor` states sum to the total stated beside them, at the
/// one place that total is stated to.
///
/// **Compared as tenths rather than as floats.** Two decimal figures are equal
/// when their rounded integers are, and asking that of `f64` directly is a
/// strict comparison of a quantity neither side arrived at the same way.
fn sums_to(floor: &[f64]) -> bool {
    let Some((total, terms)) = floor.split_last() else {
        return false;
    };
    in_tenths(terms.iter().sum::<f64>()) == in_tenths(*total)
}

/// `value` as a whole number of tenths.
fn in_tenths(value: f64) -> i64 {
    (value * 10.0).round() as i64
}

/// `value` as a whole number of hundredths, which is the precision the page
/// states these figures to.
fn to_two_places(value: f64) -> i64 {
    (value * 100.0).round() as i64
}
