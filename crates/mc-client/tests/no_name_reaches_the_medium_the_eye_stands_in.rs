//! Nothing on the path that decides what the eye stands in, or draws it,
//! compares anything against a block name.
//!
//! # Why this invariant is the one worth guarding here
//!
//! The colour a medium carries and how far it lets you see are **content**. The
//! cheap way to make the sea look like water is one comparison against
//! `base:water` in the resolver or in the uniform writer, after which the
//! shipped sea is right, every mod's own gas and acid pool draws dry, and the
//! reason is a line no mod author can reach. This spec's whole claim — that an
//! author ships a medium of their own with no engine change — is exactly what
//! that shortcut takes away, and it takes it away silently: every reading in
//! FR-2 would stay green, because they all declare the sea.
//!
//! # A declared set of sources, and what each was declared for
//!
//! Four sources, named in [`DECLARED`] with the role each plays: the resolver
//! that answers what block fills the eye's cell, the writer that puts the
//! medium into the frame's uniform, the chooser that decides what a pixel with
//! no terrain behind it is cleared to, and the shader that mixes. Between them
//! they are the whole of the path a name would have to reach to matter.
//!
//! **Each declared source carries an anchor, and a source that no longer holds
//! its anchor is not a clean source.** A file-level declaration has one failure
//! this project has shipped before: the code moves one file over, the scan goes
//! on reading an empty room, and its clean verdict is about nothing at all. So
//! the resolver is declared as the file that holds `fn eye_medium`, and the day
//! it does not, the verdict says so rather than saying the path is clean.
//!
//! # What a comparison against a name looks like, and the three shapes
//!
//! - **spelled**, as a namespaced literal — two runs of lower-case letters,
//!   digits and underscores with one colon between them, which is the shape
//!   `BlockName` parses. A source that never spells one cannot compare against
//!   one;
//! - **constructed**, through one of the doors in [`CONSTRUCTION_DOORS`]. None
//!   of these four sources builds a name: the resolver is *handed* one out of
//!   the world and passes it to the registry, and the other three never see a
//!   block at all. A source constructing one got the text from somewhere, and
//!   where is a question this path may not ask;
//! - **compared**, as an equality on a line that names `BlockName`.
//!
//! **The mention of `BlockName` is deliberately not the offence, and this is
//! the difference between a guard and a nuisance.** A correct resolver
//! necessarily handles `BlockName` — it reads one out of the cell and resolves
//! it — so a scan keyed on the mention would report a correct implementation
//! and be deleted by the first person who read it. What is looked for is a
//! *comparison*, which a correct resolver never makes.
//!
//! # An enumerated verdict, not an absence
//!
//! A scan whose sources have moved reports "no name is compared" exactly as
//! loudly as a clean path does. So the answer is one of three and every reading
//! compares the whole of it, which rejects the other two — including the one
//! meaning "there was nothing to look at" — for free.
//!
//! # What this scan does not cover, and what does
//!
//! `crates/mc-render/src` is swept whole by
//! `tests/the_mesher_and_the_renderer_name_no_block.rs`, so two of the four
//! sources here are held twice over. This one exists for the two that are not:
//! **the resolver lives in `mc-sim`, which no existing scan reads**, and the
//! shader is outside that guard's stated boundary. Being held twice is a
//! property of where the render half happened to land rather than a reason to
//! narrow either scan.
//!
//! **The shader is the weak member and it is worth saying which way.** WGSL has
//! no string type, so a shader cannot commit the spelled shape in the form
//! anything here detects; what the control below buys is that the scan is shown
//! reading a `.wgsl` file and reporting on it, so its clean verdict over the
//! real one is a reading rather than a file it was never able to say anything
//! about. What would slip past it is a shader special-casing a block by its
//! **layer index**, which carries no name at all — recorded in that sibling's
//! own header, and no more closed here than there.
//!
//! # Shape
//!
//! `tests/the_mesher_and_the_renderer_name_no_block.rs` and
//! `tests/client_names_no_content_door.rs` are the shape this follows, down to
//! comments being stripped whole-line: prose naming the block a rule was
//! written for is prose, and this spec's sources discuss the sea in comments
//! today.

mod support;

use std::error::Error;
use std::fs;
use std::path::Path;

use support::{TestResult, repository_root};

/// One source on the path, where it sits relative to the repository root, and
/// what it has to still hold for a reading of it to mean anything.
#[derive(Debug)]
struct Declared {
    at: &'static str,
    role: &'static str,
    anchors: &'static [&'static str],
}

/// Where the resolver sits, which is also the source every fixture below
/// commits its offence in.
const THE_RESOLVER: &str = "crates/mc-sim/src/world/mod.rs";

/// The four sources that decide what the eye stands in and draw it.
const DECLARED: [Declared; 4] = [
    Declared {
        at: THE_RESOLVER,
        role: "the resolver, which answers what block fills the eye's cell",
        anchors: &["fn eye_medium"],
    },
    Declared {
        at: "crates/mc-render/src/gpu/record.rs",
        role: "the uniform writer, and the chooser of the frame's clear",
        anchors: &["fn frame_uniform_bytes", "clear_color("],
    },
    Declared {
        at: "crates/mc-render/src/gpu/mod.rs",
        role: "where the clear a dry frame falls back to comes from",
        anchors: &["fn clear_color"],
    },
    Declared {
        at: "crates/mc-render/shaders/terrain.wgsl",
        role: "the stage that carries a surface toward the medium's colour",
        anchors: &["tint_color"],
    },
];

/// The doors text becomes a block name through.
///
/// Not one of them is a door any declared source goes through: the resolver is
/// handed a name and hands it on, and the three below it never see one.
const CONSTRUCTION_DOORS: [&str; 3] = ["BlockName::parse", "BlockName::new", "BlockName::from"];

/// The name a shortcut would reach for, and the one every fixture here commits.
const A_SHIPPED_NAME: &str = "base:water";

/// What a scan of the declared sources came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Not one of them spells a name, builds one, or compares against one.
    NoNameComparisonOnAnyDeclaredSource,
    /// These sources reach for a name in these ways.
    NameComparisonsFound(Vec<String>),
    /// These sources could not be read, or no longer hold what they were
    /// declared for, so nothing above could be said about them.
    DeclaredSourcesNoLongerHoldWhatTheyDecide(Vec<String>),
}

#[test]
fn no_source_that_decides_or_draws_the_eyes_medium_compares_against_a_block_name() -> TestResult {
    let verdict = verdict_over(&repository_root()?);

    assert_eq!(
        verdict,
        Verdict::NoNameComparisonOnAnyDeclaredSource,
        "the colour a medium carries and how far it lets you see are content, and the whole of \
         what makes that true is that this path cannot tell one block from another. A resolver \
         naming `{A_SHIPPED_NAME}` would draw the shipped sea correctly and every mod's own \
         medium dry, with the reason in a file no mod author can reach — and every reading of the \
         tint would stay green, because all of them declare the sea"
    );
    Ok(())
}

/// The control that distinguishes a clean path from a scan that has stopped
/// looking, driven over one comparison and no more.
///
/// A walk that broke, a needle that stopped matching, or a source that moved
/// would report a clean path forever. One offence is enough to say the scan can
/// still report one; that every shape it carries is watched is the next test's
/// job.
#[test]
fn the_same_scan_driven_over_a_source_holding_one_name_comparison_reports_that_comparison()
-> TestResult {
    let fixture = tempfile::tempdir()?;
    declared_sources_under(
        fixture.path(),
        &[(
            THE_RESOLVER,
            &format!("if held.as_str() == \"{A_SHIPPED_NAME}\" {{}}\n"),
        )],
    )?;

    let verdict = verdict_over(fixture.path());

    assert_eq!(
        verdict,
        Verdict::NameComparisonsFound(vec![format!(
            "{THE_RESOLVER} spells the name `{A_SHIPPED_NAME}`"
        )]),
        "an empty answer and an answer nobody could look for are the same picture to a reading \
         that only ever asserts an absence, and whoever has to repair a resolver that reached for \
         a block by name needs the source and the spelling in front of them"
    );
    Ok(())
}

/// Every shape the scan carries, committed at once.
///
/// A shape no fixture ever commits is a shape nobody has watched match
/// anything: mistype one and it reports a clean path for as long as it stands
/// there, which is the failure this whole file is about, one level up. The
/// expected report is derived from the same list the scan reads, so a door
/// added without a fixture committing it fails here rather than standing
/// unwatched.
#[test]
fn the_same_scan_reports_every_shape_of_reaching_for_a_name_that_it_carries() -> TestResult {
    let fixture = tempfile::tempdir()?;
    declared_sources_under(
        fixture.path(),
        &[(THE_RESOLVER, &a_source_reaching_every_way())],
    )?;

    let verdict = verdict_over(fixture.path());

    assert_eq!(
        verdict,
        Verdict::NameComparisonsFound(every_shape_reported()),
        "the scan carries a spelled name, three construction doors and an equality, and each of \
         them has to be watched matching something: a mistyped needle and a needle nothing ever \
         commits are the same green"
    );
    Ok(())
}

/// The trap this scan is shaped around, and the reason it is not keyed on the
/// mention of `BlockName`.
///
/// A correct resolver reads a name out of the cell and hands it to the
/// registry, so it names the type in its signature and in the pattern it
/// matches. A scan reporting *that* would report a correct implementation as a
/// violation, which is worse than having no scan at all: the repair a reader
/// would reach for is deleting the guard.
#[test]
fn a_source_handling_the_type_a_name_has_is_not_a_source_comparing_against_one() -> TestResult {
    let fixture = tempfile::tempdir()?;
    declared_sources_under(fixture.path(), &[(THE_RESOLVER, &a_correct_resolver())])?;

    let verdict = verdict_over(fixture.path());

    assert_eq!(
        verdict,
        Verdict::NoNameComparisonOnAnyDeclaredSource,
        "resolving a name the world handed you is the opposite of deciding which name it should \
         have been, and a guard that could not tell those apart would be reporting the correct \
         implementation of the thing it exists to protect"
    );
    Ok(())
}

/// The comment strip, which this guard cannot do without.
///
/// These sources discuss the sea in prose today — which block a bound came
/// from, what the medium is for — and a shader's prose is a bare `//` with no
/// doc form to distinguish it. Without the strip the guard would be
/// unsatisfiable for a reason that has nothing to do with the invariant, which
/// is the state in which somebody deletes the guard rather than the offence.
#[test]
fn a_name_written_in_a_comment_is_not_a_comparison_against_it() -> TestResult {
    let fixture = tempfile::tempdir()?;
    declared_sources_under(
        fixture.path(),
        &[
            (THE_RESOLVER, &a_resolver_explaining_itself()),
            (
                DECLARED[3].at,
                &format!("// The medium `\"{A_SHIPPED_NAME}\"` declares.\n"),
            ),
        ],
    )?;

    let verdict = verdict_over(fixture.path());

    assert_eq!(
        verdict,
        Verdict::NoNameComparisonOnAnyDeclaredSource,
        "a source explaining which block a rule was written for is a source documenting the seam, \
         which is the opposite of one crossing it — and a shader's prose has no doc form to be \
         told apart by, so the strip is whole-line or it is nothing"
    );
    Ok(())
}

/// The vacuity control, in both of its directions, and the reason the verdict
/// is enumerated at all.
///
/// A source that has moved and a source that no longer holds the thing it was
/// declared for both leave a scan reading nothing it can report on — which is
/// exactly what a clean path looks like. The two must never compare equal, and
/// the verdict has to name which source it is, because "somewhere on this path"
/// is not something anybody can act on.
#[test]
fn a_scan_that_can_no_longer_reach_a_declared_source_says_which_one_rather_than_reporting_it_clean()
-> TestResult {
    let gone = tempfile::tempdir()?;
    let hollowed = tempfile::tempdir()?;
    declared_sources_under(hollowed.path(), &[])?;
    fs::write(
        hollowed.path().join(THE_RESOLVER),
        "fn something_else() {}\n",
    )?;

    let reported = [verdict_over(gone.path()), verdict_over(hollowed.path())];

    assert_eq!(
        reported,
        [
            Verdict::DeclaredSourcesNoLongerHoldWhatTheyDecide(
                DECLARED.iter().map(unreadable).collect()
            ),
            Verdict::DeclaredSourcesNoLongerHoldWhatTheyDecide(vec![format!(
                "{THE_RESOLVER} ({}) no longer holds `fn eye_medium`",
                DECLARED[0].role
            )]),
        ],
        "a file that has gone and a file that no longer holds what it was declared for are the \
         two ways this scan stops being able to see, and both look identical to a clean path from \
         the outside"
    );
    Ok(())
}

/// A resolver committing every shape the scan carries.
fn a_source_reaching_every_way() -> String {
    format!(
        "let sea = BlockName::new(text)?;\n\
         let other = BlockName::from(text);\n\
         if held == BlockName::parse(\"{A_SHIPPED_NAME}\")? {{}}\n"
    )
}

/// A resolver shaped the way a correct one is: handed a name, it resolves it.
fn a_correct_resolver() -> String {
    String::from(
        "fn block_at(&self) -> Option<Contents<&BlockName>> { self.held }\n\
         let Contents::Holds(name) = world.block_at(containing(at))? else { return None; };\n\
         world.registry().resolve(name).ok()?.tint\n",
    )
}

/// A resolver whose prose names the block its rule was written for, in each of
/// the three comment forms Rust has.
fn a_resolver_explaining_itself() -> String {
    format!(
        "//! Where `\"{A_SHIPPED_NAME}\"` is resolved.\n\
         /// Written when `\"{A_SHIPPED_NAME}\"` was the only one, past `BlockName::parse`.\n\
         // if held.as_str() == \"{A_SHIPPED_NAME}\" is what this deliberately is not.\n\
         pub fn resolved() {{}}\n"
    )
}

/// What the scan reports of [`a_source_reaching_every_way`]: the name it
/// spells, then every door in the order the scan carries them, then the line it
/// compares on.
fn every_shape_reported() -> Vec<String> {
    [format!("{THE_RESOLVER} spells the name `{A_SHIPPED_NAME}`")]
        .into_iter()
        .chain(
            CONSTRUCTION_DOORS
                .iter()
                .map(|door| format!("{THE_RESOLVER} names `{door}`")),
        )
        .chain([format!(
            "{THE_RESOLVER} compares against a block name: \
             `if held == BlockName::parse(\"{A_SHIPPED_NAME}\")? {{}}`"
        )])
        .collect()
}

/// What the declared sources under `root` came to.
///
/// **An unreadable source is a verdict rather than an error**, for the same
/// reason reading nothing is not "no name is compared": a scan that cannot look
/// has to say so in the answer it gives, not in a failure that ends the test
/// before its assertion ran.
fn verdict_over(root: &Path) -> Verdict {
    let mut lost = Vec::new();
    let mut found = Vec::new();
    for source in &DECLARED {
        read_declared(root, source, &mut lost, &mut found);
    }
    if !lost.is_empty() {
        return Verdict::DeclaredSourcesNoLongerHoldWhatTheyDecide(lost);
    }
    if found.is_empty() {
        return Verdict::NoNameComparisonOnAnyDeclaredSource;
    }
    Verdict::NameComparisonsFound(found)
}

/// Reads one declared source, recording what it can no longer be read for and
/// every way it reaches for a name.
fn read_declared(root: &Path, source: &Declared, lost: &mut Vec<String>, found: &mut Vec<String>) {
    let Ok(held) = fs::read_to_string(root.join(source.at)) else {
        lost.push(unreadable(source));
        return;
    };
    let text = production_text(&held);
    lost.extend(
        source
            .anchors
            .iter()
            .filter(|anchor| !text.contains(**anchor))
            .map(|anchor| format!("{} ({}) no longer holds `{anchor}`", source.at, source.role)),
    );
    found.extend(reaching_for_a_name_in(&text, source.at));
}

/// How a source nobody could read is reported.
fn unreadable(source: &Declared) -> String {
    format!("{} ({}) could not be read", source.at, source.role)
}

/// Every way `text` reaches for a block name, said in the order the shapes are
/// looked for.
fn reaching_for_a_name_in(text: &str, at: &str) -> Vec<String> {
    let mut found: Vec<String> = namespaced_names_in(text)
        .into_iter()
        .map(|name| format!("{at} spells the name `{name}`"))
        .collect();
    found.extend(
        CONSTRUCTION_DOORS
            .iter()
            .filter(|door| text.contains(**door))
            .map(|door| format!("{at} names `{door}`")),
    );
    found.extend(
        compared_against_a_name_in(text)
            .into_iter()
            .map(|line| format!("{at} compares against a block name: `{line}`")),
    );
    found
}

/// Every line of `text` that tests an equality and names `BlockName` in the
/// same breath, collapsed to single spaces so a report reads the same however
/// the source was laid out.
fn compared_against_a_name_in(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.contains("BlockName") && (line.contains("==") || line.contains("!=")))
        .map(|line| line.split_whitespace().collect::<Vec<&str>>().join(" "))
        .collect()
}

/// Every distinct namespaced name `text` spells inside a string literal, in the
/// order it first spells them.
fn namespaced_names_in(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for literal in text.split('"').skip(1).step_by(2) {
        if is_a_namespaced_name(literal) && !found.iter().any(|seen| seen == literal) {
            found.push(literal.to_owned());
        }
    }
    found
}

/// Whether `literal` is spelled the way a block name is: two non-empty runs of
/// lower-case letters, digits and underscores with one colon between them.
fn is_a_namespaced_name(literal: &str) -> bool {
    let [namespace, name] = literal.split(':').collect::<Vec<&str>>()[..] else {
        return false;
    };
    [namespace, name].iter().all(|part| {
        !part.is_empty()
            && part.chars().all(|letter| {
                letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '_'
            })
    })
}

/// A source's text with its whole-line comments removed, in the one form Rust
/// and WGSL share.
fn production_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Writes a copy of every declared source under `root`, each holding what it
/// was declared to hold, with whatever `added` says appended to it.
///
/// **Written to hold its own anchors, and that is legitimate here and nowhere
/// else.** A control fixture's whole purpose is to be the thing the scan should
/// report or pass over, and a fixture whose sources looked unreadable would
/// reach the vacuity arm before any offence in it was read.
///
/// # Errors
///
/// Returns an error if a declared source has no directory above it, or if the
/// tree cannot be written.
fn declared_sources_under(root: &Path, added: &[(&str, &str)]) -> Result<(), Box<dyn Error>> {
    for source in &DECLARED {
        let at = root.join(source.at);
        fs::create_dir_all(
            at.parent()
                .ok_or("a declared source with no directory above it")?,
        )?;
        let extra: String = added
            .iter()
            .filter(|(to, _)| *to == source.at)
            .map(|(_, text)| *text)
            .collect();
        fs::write(at, format!("{}\n{extra}", source.anchors.join("\n")))?;
    }
    Ok(())
}
