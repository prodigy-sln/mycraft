//! Everything a declaration can say about how much light its block stops that
//! the engine will not take, and the sentence each of those refusals owes its
//! author.
//!
//! A refused declaration costs a mod author a line they can read. **A coerced one
//! costs them a block that is quietly the wrong thing to look at** — a wall where
//! they wrote glass, or a pane of nothing where they wrote a tint — with no
//! symptom to search for, which is why every value below is refused rather than
//! clamped, rounded or parsed.
//!
//! # This is the first field with a ceiling anybody can reach
//!
//! The two numbers a medium states have a floor and, in practice, no ceiling: the
//! only thing above them is a width nobody types on purpose, and a value past it
//! is refused as **not finite** rather than as too large. A degree of opacity
//! stops at `1.0`, which is a number an author will genuinely write — reaching
//! for "more than opaque" the way they reach for `math.huge` when they want "as
//! fast as possible". So the vocabulary gains a fourth sentence here, and the
//! order the four are asked in becomes observable for the first time.
//!
//! # The order of the four branches is the subject, not an implementation detail
//!
//! Wrong kind, then not finite, then below the floor, then above the ceiling.
//! Both of the last two fixtures exist to pin that order, and each of them fails
//! in a *different* direction if the order is wrong:
//!
//! - `0/0` is a NaN, and `NaN >= 0.0` is false. A floor test reached first
//!   refuses it with a sentence about zero and sends its author looking for a
//!   minus sign they never wrote.
//! - `math.huge` is an infinity, and `infinity > 1.0` is true. A ceiling test
//!   reached first refuses it with a sentence about `1.0` and teaches an author
//!   that the fix is a smaller number, when what they wrote is not a number at
//!   all.
//!
//! Neither is visible to a reading that asks only whether the root was refused,
//! and the second is new: it could not arise while no field had a ceiling.
//!
//! # The cross-field refusal, and why it is the loader's first
//!
//! `occludes = true` says a block hides what lies beyond it; an opacity below
//! `1.0` says light passes through it. A declaration stating both says two things
//! that cannot both be honoured, and the engine resolves it in the mesher — where
//! the neighbouring face is never emitted, so the see-through block has nothing
//! behind it to show. Refusing the pair is what stops a mod author shipping a
//! window they can never see out of and finding no error anywhere.
//!
//! It is raised on `opacity` and names both fields, because the field an author
//! just added is the one they are looking at. Both fields are read before it is
//! raised, so it is a statement about the declaration rather than about whichever
//! line the loader happened to reach first.
//!
//! # The cause travels whole
//!
//! A refusal naming the field and then saying something else entirely about it is
//! exactly what a `contains` on the field name cannot see, and it is the likelier
//! of the two failures while a field is new — because an **unrecognised** key is
//! refused by name too, and that is the refusal this whole file gets on a tree
//! where the loader has never heard of the field.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, QUARTZ, blaming, declaration_of, judged, raw_field, text_field,
};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// The key a declaration states how much light it stops in.
const OPACITY_FIELD: &str = "opacity";

/// The key a declaration states hiding its neighbours in.
const OCCLUDES_FIELD: &str = "occludes";

/// A degree written as text that names the number it is not.
///
/// The mistake a field whose value is a number invites from anybody used to a
/// format where everything is a string, and `half` rather than `'0.5'` because
/// it is the spelling somebody arrives at by describing what they want instead
/// of measuring it. Parsing it is refused for the reason coercion is refused
/// everywhere else on this declaration: an author who wrote a word and got a
/// number learns nothing, and the next word they write is one nothing parses.
const A_DEGREE_WRITTEN_AS_A_STRING: &str = "'half'";

/// A degree a tenth below the floor.
///
/// Just below rather than far below, because a loader whose floor had drifted to
/// some other negative number satisfies a fixture at `-1000` exactly as a correct
/// one does.
const A_DEGREE_BELOW_THE_FLOOR: &str = "-0.1";

/// A degree half again past the ceiling.
///
/// The value somebody writes reaching for "more opaque than opaque", which is
/// what a percentage-shaped intuition produces on a scale that runs to one.
const A_DEGREE_PAST_THE_CEILING: &str = "1.5";

/// The expression Luau evaluates to a NaN.
const A_DEGREE_THAT_IS_NOT_A_NUMBER: &str = "0/0";

/// Luau's own name for an infinity.
///
/// Stated as `math.huge` rather than as a division, because it is the spelling
/// somebody arrives at deliberately: a NaN is a mistake, and this is a wish —
/// and on a field with a ceiling it is the wish whose refusal must not be the
/// ceiling's sentence.
const A_DEGREE_WITHOUT_BOUND: &str = "math.huge";

/// A degree that lets light through, for the fixture that also hides what is
/// behind it.
const A_DEGREE_THAT_PASSES_LIGHT: &str = "0.5";

/// A degree that stops all of it, which every ordinary block has and states
/// nothing about.
const A_DEGREE_THAT_PASSES_NONE: &str = "1.0";

/// What a refusal about this field owes: the file, the block, the field, and the
/// sentence a mod author reads.
///
/// The cause travels whole rather than as a substring check, for the reason this
/// file's header gives.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    blamed: Blamed,
    cause: String,
}

/// What a declaration stating a degree and an occlusion did.
///
/// **An enumerated verdict, because `refused` alone cannot answer the cross-field
/// question.** The refusal an unrecognised field raises also names a field, also
/// says a sentence, and also registers nothing — and it is the answer this tree
/// gives before the loader has heard of the field at all. A verdict that asked
/// only whether something was refused would be green over a loader that never
/// read either line.
#[derive(Debug, PartialEq, Eq)]
enum WhatTheDeclarationDid {
    /// Accepted, and the block is in the registry.
    Registered,
    /// Refused, naming this and saying that.
    Refused(Refusal),
    /// Accepted, and the block is not in the registry — which is neither of the
    /// two answers a load may give and is reported rather than folded into one.
    AcceptedWithoutTheBlock,
}

/// A root whose one declaration states `field` as `stated` and is otherwise well
/// formed.
///
/// The shape every fixture here takes: a declaration that would register, and one
/// line in it that must stop it. A fixture built any other way leaves it open
/// whether the refusal was about that line at all.
fn root_stating(directory: &TempDir, field: &str, stated: &str) -> Result<PathBuf, Box<dyn Error>> {
    root_of(directory, &[(field, stated)])
}

/// A root whose one declaration states each of `fields` and is otherwise well
/// formed.
fn root_of(directory: &TempDir, fields: &[(&str, &str)]) -> Result<PathBuf, Box<dyn Error>> {
    let mut declared = vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "true"),
    ];
    for (field, stated) in fields {
        declared.push(raw_field(field, stated));
    }
    content_root(directory, &[(AMBER_FILE, declaration_of(&declared))])
}

/// What the root at `root` refused, and what it said about it.
fn refusal_of(root: &Path) -> Refusal {
    let (blamed, cause) = judged(root, AMBER_FILE);
    Refusal { blamed, cause }
}

/// A refusal blaming [`AMBER`]'s opacity with `cause`.
fn blaming_the_degree(cause: &str) -> Refusal {
    Refusal {
        blamed: Blamed::Declaration(blaming(AMBER, OPACITY_FIELD)),
        cause: cause.to_owned(),
    }
}

/// What the root at `root` did with the declaration it holds.
fn what_the_declaration_did(root: &Path) -> WhatTheDeclarationDid {
    let mut registry = BlockRegistry::new();
    if registry
        .apply(&LuauFileDefinitionSource::new(root))
        .is_err()
    {
        return WhatTheDeclarationDid::Refused(refusal_of(root));
    }
    if BlockName::parse(AMBER).is_ok_and(|name| registry.resolve(&name).is_ok()) {
        return WhatTheDeclarationDid::Registered;
    }
    WhatTheDeclarationDid::AcceptedWithoutTheBlock
}

/// The sentence a declaration stating both halves of the contradiction owes.
///
/// Written out whole rather than assembled from the two field names, because it
/// is the one sentence on this declaration that has to explain a *pairing* — a
/// reader who is told only which two fields are involved still has to work out
/// which of them to change, and the clause after the colon is what tells them
/// what the engine would otherwise have to decide on their behalf.
const A_BLOCK_CANNOT_BOTH_PASS_LIGHT_AND_HIDE_WHAT_IS_BEYOND_IT: &str = "`opacity` below one cannot be stated with `occludes = true`: a block light passes through \
     cannot also hide what lies beyond it";

/// The sentence owed to a block that never wrote the second line at all.
///
/// **A different sentence, because the remedy is different.** The one above
/// names a line to delete; this one names a line to add, and it has to say
/// where the occlusion came from — an author who wrote `solid = true` and
/// `opacity = 0.5` has no `occludes` in their file to find, and a refusal
/// quoting one would send them looking for something that is not there.
/// `solid` is named for the same reason the recognised-field list is quoted
/// back at a misspelling: the offending line is only recognisable once you can
/// see what made it one.
const A_BLOCK_THAT_OCCLUDES_BY_BEING_SOLID_CANNOT_PASS_LIGHT_EITHER: &str = "`opacity` below one cannot be stated with `occludes = true`, and this block occludes by \
     stating `solid = true` and no `occludes`: a block light passes through cannot also hide \
     what lies beyond it";

#[test]
fn a_degree_written_as_a_string_is_refused_naming_the_kind_a_number_is() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, OPACITY_FIELD, A_DEGREE_WRITTEN_AS_A_STRING)?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_degree(&format!(
            "`{OPACITY_FIELD}` must be a number, but is a string"
        )),
        "the worst available outcome for a word where a number belongs is falling back to the \
         default, because the default is `1.0` — so the block would draw exactly as if the line \
         had never been written and its author would never learn it did nothing. The kind found \
         is quoted as well as the kind expected, because it is what tells an author which of \
         their lines is the one: `half` and `0.5` sit a quotation mark apart and only one of \
         them is a number"
    );
    Ok(())
}

#[test]
fn a_degree_below_zero_is_refused_naming_the_field_and_the_floor() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, OPACITY_FIELD, A_DEGREE_BELOW_THE_FLOOR)?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_degree(&format!("`{OPACITY_FIELD}` may not be less than zero")),
        "zero already means every photon gets through, so there is nothing below it for a \
         declaration to mean — a negative degree would be a block that emits the light it was \
         given, which is not a thing this field can express and not a thing the renderer would \
         do with it. The sentence is the one every other number on this declaration uses for \
         its floor, deliberately: a field that invented its own wording for the same mistake \
         gives a mod author a second dialect to learn"
    );
    Ok(())
}

#[test]
fn a_degree_above_one_is_refused_naming_the_field_and_the_ceiling() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, OPACITY_FIELD, A_DEGREE_PAST_THE_CEILING)?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_degree(&format!("`{OPACITY_FIELD}` may not be more than one")),
        "one already means no light gets through, so there is nothing above it either, and this \
         is the first bound on this declaration a mod author can actually reach — the two \
         numbers a medium states stop at a width nobody types. Clamping is the tempting wrong \
         answer and the expensive one: `1.5` clamped to `1.0` is a block that draws correctly, \
         so the author never learns their scale runs to one and writes `100` on the next block \
         expecting a percentage. The ceiling is worded like the floor beside it, because they \
         are the two ends of one sentence a reader holds in their head at once"
    );
    Ok(())
}

#[test]
fn a_degree_that_is_not_a_number_is_refused_for_that_and_never_for_the_floor() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, OPACITY_FIELD, A_DEGREE_THAT_IS_NOT_A_NUMBER)?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_degree(&format!("`{OPACITY_FIELD}` must be a finite number")),
        "Luau evaluates `0/0` to a NaN and hands it over as a perfectly ordinary declared \
         number, so this is reachable from a content file rather than a guard against something \
         nobody can write. It has to be refused **for not being a number**: `NaN >= 0.0` is \
         false, so a floor test reached first blames the sign of a value that has no sign and \
         sends its author hunting for a minus they never wrote. The cause is compared whole, \
         which is what rejects the floor's sentence rather than merely preferring this one"
    );
    Ok(())
}

#[test]
fn a_degree_without_bound_is_refused_for_finiteness_and_never_for_the_ceiling() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, OPACITY_FIELD, A_DEGREE_WITHOUT_BOUND)?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_degree(&format!("`{OPACITY_FIELD}` must be a finite number")),
        "`math.huge` is what somebody writes reaching for `as opaque as possible`, and it is \
         the fixture the ceiling makes newly dangerous: `infinity > 1.0` is true, so a ceiling \
         test reached before the finiteness one refuses this with a sentence about `1.0` and \
         teaches its author that the fix is a smaller number — when what they wrote is not a \
         number at all and no smaller spelling of it exists. Nothing could report this before \
         opacity, because no field on this declaration had a reachable ceiling to be blamed by \
         mistake"
    );
    Ok(())
}

#[test]
fn a_block_that_passes_light_and_also_hides_what_is_behind_it_is_refused_naming_both_fields()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_of(
        &directory,
        &[
            (OPACITY_FIELD, A_DEGREE_THAT_PASSES_LIGHT),
            (OCCLUDES_FIELD, "true"),
        ],
    )?;

    assert_eq!(
        what_the_declaration_did(&root),
        WhatTheDeclarationDid::Refused(blaming_the_degree(
            A_BLOCK_CANNOT_BOTH_PASS_LIGHT_AND_HIDE_WHAT_IS_BEYOND_IT
        )),
        "these two lines ask for opposite things and the engine cannot honour both: `occludes` \
         suppresses the neighbour's meeting face, so the block that light was supposed to pass \
         through has nothing behind it left to show. Whichever way the engine broke the tie it \
         would be overruling a line somebody wrote, silently — which is why this is the \
         loader's first cross-field refusal rather than a precedence rule. It is raised on \
         `opacity` because that is the field the author just added and the one they are looking \
         at, and it names `occludes` too because a refusal blaming one half of a contradiction \
         leaves the other half unfindable"
    );
    Ok(())
}

/// What a root stating `degree` beside `occludes` did with its declaration.
fn a_root_stating(degree: &str, occludes: &str) -> Result<WhatTheDeclarationDid, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let root = root_of(
        &directory,
        &[(OPACITY_FIELD, degree), (OCCLUDES_FIELD, occludes)],
    )?;
    Ok(what_the_declaration_did(&root))
}

#[test]
fn each_half_of_that_contradiction_registers_on_its_own() -> TestResult {
    let seen_through = a_root_stating(A_DEGREE_THAT_PASSES_LIGHT, "false")?;
    let ordinary = a_root_stating(A_DEGREE_THAT_PASSES_NONE, "true")?;

    assert_eq!(
        (seen_through, ordinary),
        (
            WhatTheDeclarationDid::Registered,
            WhatTheDeclarationDid::Registered,
        ),
        "the control on the refusal above, and it is not optional: a cross-field rule that \
         over-fires is far worse than one that does not exist. The second of these two is the \
         **ordinary** declaration — an opaque block that hides what is behind it, which is what \
         every shipped block and every block anybody has ever written is — so a rule reading \
         `occludes = true` as the offence on its own would refuse every content root in \
         existence. The first is the whole point of the feature and would be refused by a rule \
         that read a degree below one as the offence on its own"
    );
    Ok(())
}

#[test]
fn a_solid_block_that_passes_light_is_refused_and_told_which_line_makes_it_occlude() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, OPACITY_FIELD, A_DEGREE_THAT_PASSES_LIGHT)?;

    assert_eq!(
        what_the_declaration_did(&root),
        WhatTheDeclarationDid::Refused(blaming_the_degree(
            A_BLOCK_THAT_OCCLUDES_BY_BEING_SOLID_CANNOT_PASS_LIGHT_EITHER
        )),
        "this is glass — solid, see-through, and the block the spec's second user story names — \
         and it is what a mod author's first attempt at the feature looks like. It has to be \
         refused: `occludes` means the block's own solidity where a declaration says nothing, so \
         this one hides what is behind it and the translucent face would draw over culled \
         geometry with no symptom to search for. **And it may not be refused in the sentence \
         next door.** That one quotes an `occludes = true` this file does not contain, so an \
         author would grep for it, find nothing, and conclude the engine is confused. Naming \
         `solid` is what turns the refusal into an instruction: state `occludes = false` and \
         the block is the one they meant to write"
    );
    Ok(())
}
