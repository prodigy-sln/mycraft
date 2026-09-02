//! Everything a declaration can say about the medium its block is that the
//! engine will not take, and the sentence each of those refusals owes its
//! author.
//!
//! A refused declaration costs a mod author a line they can read. **A coerced
//! one costs them a medium that is quietly the wrong thing to look through** —
//! a colour whose channels were shuffled, a strength silently dropped, a
//! distance clamped to something they never wrote — with no symptom to search
//! for, which is why every value below is refused rather than parsed into
//! something near it.
//!
//! # Four causes for the colour, because four scenarios tell them apart
//!
//! Wrong kind, not one of the two accepted forms, and an eight-digit form whose
//! alpha is not `FF` are three different mistakes with three different remedies,
//! and the fourth is the distance's own wrong kind. Folding any two of them into
//! one sentence sends half the authors who hit it to the wrong line: somebody who
//! wrote `#3A6EA580` reaching for a weaker tint has written a well-formed colour
//! and needs to be told where strength actually lives, and telling them their
//! colour is malformed teaches them to distrust a form the loader accepts.
//!
//! # The exclusive floor is this declaration's first
//!
//! Every other number on a declaration admits zero: a resistance of zero is
//! "unaffected" and a degree of zero is a pane with no glass in it. A medium
//! reaching its full strength at *no distance at all* hides everything including
//! the inside of the eye, which is a different claim from any this field admits
//! and not a weaker one — and the reciprocal the draw path carries would not be
//! finite. So the floor is `> 0` where every other floor is `>= 0`, and it is
//! refused in words of its own rather than in the shared "may not be less than
//! zero", which would be true of a value this field accepts nowhere.
//!
//! # The order of the branches is the subject, not an implementation detail
//!
//! Finiteness is asked before the floor, and both fixtures of that pair fail in
//! opposite directions if the order is wrong: `math.huge` is an infinity and
//! `infinity > 0.0` is **true**, so a floor test reached first lets it through
//! entirely; `0/0` is a NaN and `NaN > 0.0` is **false**, so a floor test reached
//! first refuses it with a sentence about zero and sends its author looking for a
//! minus sign they never wrote. One value escapes and the other is misattributed,
//! which is why the scenario names both.
//!
//! # Both-or-neither is decided after both reads
//!
//! A declaration stating one of the two fields is refused naming the **missing**
//! one as the thing to add, in two sentences rather than one — the remedies are
//! different and each has to name the line the author already has, or they will
//! grep for a field their file does not contain and conclude the engine is
//! confused.
//!
//! # The cause travels whole
//!
//! A refusal naming the field and then saying something else entirely about it is
//! exactly what a `contains` on the field name cannot see, and it is the likelier
//! of the two failures while a field is new — because an **unrecognised** key is
//! refused by name too, and that is the refusal this whole file gets on a tree
//! where the loader has never heard of either field.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, QUARTZ, blaming, declaration_of, judged, raw_field, text_field,
};
use tempfile::TempDir;

/// The key a declaration states the colour of the medium it is in.
const TINT_FIELD: &str = "tint";

/// The key a declaration states how far that medium lets an eye see in.
const TINT_DISTANCE_FIELD: &str = "tint_distance";

/// A colour stated as the number an author reaches for when they think of a
/// colour as a value rather than as text.
const A_COLOUR_STATED_AS_A_NUMBER: &str = "5";

/// Six digits, two of which are not digits at all.
///
/// `G` rather than a wilder character because it is the letter a hexadecimal
/// alphabet stops one short of, so it is the mistake somebody makes counting
/// letters rather than the mistake of typing nonsense.
const A_COLOUR_WITH_A_DIGIT_THAT_IS_NOT_ONE: &str = "'#GG0000'";

/// Six perfectly good digits with nothing in front of them.
///
/// The spelling a mod author arrives at from a palette tool that reports a
/// colour without its lead, and the one whose refusal has to say what is
/// missing rather than that the value is unrecognisable.
const A_COLOUR_WITH_NO_LEADING_HASH: &str = "'3A6EA5'";

/// Seven digits: one short of the eight-digit form and one past the six.
///
/// The length that tells the two accepted forms apart, and the one a refusal
/// naming only one of them would misdescribe.
const A_COLOUR_ONE_DIGIT_SHORT: &str = "'#3A6EA'";

/// Eight well-formed digits whose alpha is half.
///
/// **Not a malformed colour**: every character is a hexadecimal digit and the
/// length is one the loader accepts. What is wrong with it is that it states a
/// strength, and strength is the distance's job — so its refusal has to name
/// where strength lives rather than describe the form.
const A_COLOUR_STATING_HALF_AN_ALPHA: &str = "'#3A6EA580'";

/// A colour the loader accepts, for the fixtures whose subject is the distance
/// or the other field's absence.
const A_COLOUR_THE_LOADER_ACCEPTS: &str = "'#3A6EA5'";

/// A distance at the floor, which is outside it.
const A_DISTANCE_OF_NO_DISTANCE: &str = "0.0";

/// A distance a whole block under it.
const A_DISTANCE_BELOW_THE_FLOOR: &str = "-1.0";

/// Luau's own name for an infinity.
///
/// The spelling somebody arrives at reaching for "you can see forever through
/// this", and the one an exclusive floor makes newly dangerous: `infinity > 0.0`
/// is true, so a floor reached before the finiteness check admits it outright.
const A_DISTANCE_WITHOUT_BOUND: &str = "math.huge";

/// The expression Luau evaluates to a NaN.
const A_DISTANCE_THAT_IS_NOT_A_NUMBER: &str = "0/0";

/// A distance stated as the word an author writes when they describe what they
/// want instead of measuring it.
const A_DISTANCE_WRITTEN_AS_A_STRING: &str = "'far'";

/// A distance the loader accepts, for the fixture whose subject is the colour.
const A_DISTANCE_THE_LOADER_ACCEPTS: &str = "12.0";

/// The sentence a colour of the wrong kind owes.
///
/// The kind found is quoted as well as the kind expected, because it is what
/// tells an author which of their lines is the one: `5` and `'#050505'` are a
/// quotation mark and four characters apart, and only one of them is a colour.
const A_TINT_MUST_BE_A_COLOUR_STRING: &str = "`tint` must be a colour string, but is a number";

/// The sentence a colour that is not one of the two forms owes.
///
/// **Both forms are named, and the case is named with them.** A refusal quoting
/// one form teaches an author that the other is wrong, and the two dialects are
/// both already written in this tree — so half of everyone copying a shipped
/// file would be told the file they copied is malformed.
const A_TINT_MUST_BE_WRITTEN_ONE_OF_TWO_WAYS: &str =
    "`tint` must be written `#RRGGBB` or `#RRGGBBAA`, in upper case or lower";

/// The sentence an eight-digit colour whose alpha is not `FF` owes.
///
/// It names where strength lives rather than describing the form, because the
/// form is correct. An author who is told this line is malformed edits it into
/// the six-digit form, loses the strength they were reaching for, and never
/// learns the field that carries it.
const A_TINT_STATES_NO_ALPHA: &str = "`tint` states no alpha: how strongly a medium acts is \
     `tint_distance`, so an eight-digit colour must end `FF`";

/// The sentence a distance at or below the floor owes.
///
/// **Its own words rather than the shared "may not be less than zero"**, which
/// is true of a value this field does not accept: zero is outside this range and
/// inside every other numeric range on the declaration.
const A_DISTANCE_MUST_BE_GREATER_THAN_ZERO: &str = "`tint_distance` must be greater than zero";

/// The sentence a distance that is not a finite number owes, which is the one
/// every other number on this declaration uses.
const A_DISTANCE_MUST_BE_FINITE: &str = "`tint_distance` must be a finite number";

/// The sentence a distance of the wrong kind owes, inherited unchanged from the
/// loader's only numeric reader.
const A_DISTANCE_MUST_BE_A_NUMBER: &str = "`tint_distance` must be a number, but is a string";

/// The sentence a colour with no distance owes.
///
/// It names the field to add and the field that requires it, so an author who
/// greps for `tint_distance` and finds nothing has already been told why.
const A_DISTANCE_IS_REQUIRED_BESIDE_A_TINT: &str = "`tint_distance` is required beside `tint`: a \
     colour with no distance does not say how far this medium lets an eye see";

/// The sentence a distance with no colour owes.
///
/// **A different sentence, because the remedy is different**: this one names a
/// colour to add where the one above names a distance, and a single sentence
/// covering both would have to be vague about which line is missing — which is
/// the whole of what the author needs.
const A_TINT_IS_REQUIRED_BESIDE_A_DISTANCE: &str = "`tint` is required beside `tint_distance`: a \
     distance with no colour does not say what this medium carries a view toward";

/// What a refusal about these fields owes: the file, the block, the field, and
/// the sentence a mod author reads.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    blamed: Blamed,
    cause: String,
}

/// A root whose one declaration states each of `fields` and is otherwise well
/// formed.
///
/// The shape every fixture here takes: a declaration that would register, and
/// one line in it that must stop it. A fixture built any other way leaves it
/// open whether the refusal was about that line at all.
fn root_of(directory: &TempDir, fields: &[(&str, &str)]) -> Result<PathBuf, Box<dyn Error>> {
    let mut declared = vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "false"),
        raw_field("occludes", "false"),
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

/// A refusal blaming [`AMBER`]'s `field` with `cause`.
fn blaming_the(field: &str, cause: &str) -> Refusal {
    Refusal {
        blamed: Blamed::Declaration(blaming(AMBER, field)),
        cause: cause.to_owned(),
    }
}

/// What a root stating a colour of `stated` beside an acceptable distance
/// refused.
fn refusing_the_colour(stated: &str) -> Result<Refusal, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let root = root_of(
        &directory,
        &[
            (TINT_FIELD, stated),
            (TINT_DISTANCE_FIELD, A_DISTANCE_THE_LOADER_ACCEPTS),
        ],
    )?;
    Ok(refusal_of(&root))
}

/// What a root stating a distance of `stated` beside an acceptable colour
/// refused.
fn refusing_the_distance(stated: &str) -> Result<Refusal, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let root = root_of(
        &directory,
        &[
            (TINT_FIELD, A_COLOUR_THE_LOADER_ACCEPTS),
            (TINT_DISTANCE_FIELD, stated),
        ],
    )?;
    Ok(refusal_of(&root))
}

#[test]
fn a_colour_stated_as_a_number_is_refused_naming_the_kind_a_colour_is() -> TestResult {
    assert_eq!(
        refusing_the_colour(A_COLOUR_STATED_AS_A_NUMBER)?,
        blaming_the(TINT_FIELD, A_TINT_MUST_BE_A_COLOUR_STRING),
        "a colour is text with a `#` in front of it, and the mistake this refuses is the one \
         somebody makes who thinks of a colour as a value — a packed integer, an index into a \
         palette. The worst available outcome is falling back to no tint at all, because that \
         is what an unstated field means: the block would draw exactly as if the line had never \
         been written and its author would never learn it did nothing. `a colour string` rather \
         than `a string` is what tells them the remedy is not merely quotation marks"
    );
    Ok(())
}

#[test]
fn a_colour_that_is_neither_accepted_form_is_refused_naming_both_of_them() -> TestResult {
    let not_a_digit = refusing_the_colour(A_COLOUR_WITH_A_DIGIT_THAT_IS_NOT_ONE)?;
    let no_hash = refusing_the_colour(A_COLOUR_WITH_NO_LEADING_HASH)?;
    let one_short = refusing_the_colour(A_COLOUR_ONE_DIGIT_SHORT)?;

    let expected = blaming_the(TINT_FIELD, A_TINT_MUST_BE_WRITTEN_ONE_OF_TWO_WAYS);
    assert_eq!(
        (not_a_digit, no_hash, one_short),
        (
            blaming_the(TINT_FIELD, A_TINT_MUST_BE_WRITTEN_ONE_OF_TWO_WAYS),
            blaming_the(TINT_FIELD, A_TINT_MUST_BE_WRITTEN_ONE_OF_TWO_WAYS),
            expected,
        ),
        "three ways of writing a colour wrong — a character outside the alphabet, a missing \
         lead, and a length between the two accepted ones — and one sentence, because the \
         remedy is the same for all three: write one of the two forms. Both forms are named \
         because both are already written in this tree, in two directories, each behind a \
         reader claiming to be the only one; a refusal quoting a single form would tell half \
         of everybody copying a shipped file that the file they copied is malformed. The three \
         are read in one comparison, so a loader that accepted any of them, or refused one of \
         them differently, is reported rather than half-reported"
    );
    Ok(())
}

#[test]
fn an_eight_digit_colour_stating_a_partial_alpha_is_told_where_strength_lives() -> TestResult {
    let stating_an_alpha = refusing_the_colour(A_COLOUR_STATING_HALF_AN_ALPHA)?;
    let malformed = refusing_the_colour(A_COLOUR_WITH_A_DIGIT_THAT_IS_NOT_ONE)?;
    let told_apart = stating_an_alpha.cause != malformed.cause;

    assert_eq!(
        (stating_an_alpha, told_apart),
        (blaming_the(TINT_FIELD, A_TINT_STATES_NO_ALPHA), true),
        "every character of this value is a hexadecimal digit and its length is one the loader \
         accepts, so it is not a malformed colour and must not be refused as one. What is \
         wrong with it is that it states a strength, and this field states none: how strongly \
         a medium acts is how far it lets you see, which is the other line. An author told \
         their colour is malformed edits it into six digits, loses the strength they were \
         reaching for, and never learns which field carries it. The two causes are compared \
         against each other as well as against the text, because `distinct from the one raised \
         against a malformed colour` is the half of this a single expected string cannot state"
    );
    Ok(())
}

#[test]
fn a_distance_at_or_under_zero_is_refused_naming_the_field_and_its_own_floor() -> TestResult {
    let no_distance = refusing_the_distance(A_DISTANCE_OF_NO_DISTANCE)?;
    let under = refusing_the_distance(A_DISTANCE_BELOW_THE_FLOOR)?;

    let expected = blaming_the(TINT_DISTANCE_FIELD, A_DISTANCE_MUST_BE_GREATER_THAN_ZERO);
    assert_eq!(
        (no_distance, under),
        (
            blaming_the(TINT_DISTANCE_FIELD, A_DISTANCE_MUST_BE_GREATER_THAN_ZERO),
            expected,
        ),
        "this is the first exclusive floor on the declaration and both ends of the pair matter: \
         zero is the value every other number here accepts, so a reader that reached for the \
         shared bound admits it, and a negative distance is what somebody writes having decided \
         the ramp runs the other way. A medium reaching full strength at no distance hides \
         everything including the inside of the eye — and the reciprocal the draw path carries \
         would not be finite, so the bound has two independent reasons. The sentence is the \
         field's own rather than `may not be less than zero`, which would be a true statement \
         about a value this field refuses"
    );
    Ok(())
}

#[test]
fn a_distance_that_is_not_a_finite_number_is_refused_for_that_and_never_for_the_floor() -> TestResult
{
    let without_bound = refusing_the_distance(A_DISTANCE_WITHOUT_BOUND)?;
    let not_a_number = refusing_the_distance(A_DISTANCE_THAT_IS_NOT_A_NUMBER)?;

    let expected = blaming_the(TINT_DISTANCE_FIELD, A_DISTANCE_MUST_BE_FINITE);
    assert_eq!(
        (without_bound, not_a_number),
        (
            blaming_the(TINT_DISTANCE_FIELD, A_DISTANCE_MUST_BE_FINITE),
            expected,
        ),
        "both values are reachable from a content file and the pair is what catches a wrong \
         ordering whichever way it is wrong. `math.huge` is what somebody writes reaching for \
         `you can see forever through this`, and `infinity > 0.0` is **true** — so a floor \
         reached before the finiteness check lets it through entirely and hands the draw path \
         a reciprocal of zero. `0/0` is a NaN and `NaN > 0.0` is **false**, so the same wrong \
         ordering refuses it with a sentence about zero and sends its author hunting for a \
         minus sign they never wrote. One escapes and the other is misattributed; a fixture \
         holding either alone can only see one of those"
    );
    Ok(())
}

#[test]
fn a_distance_written_as_a_word_is_refused_naming_the_kind_a_number_is() -> TestResult {
    assert_eq!(
        refusing_the_distance(A_DISTANCE_WRITTEN_AS_A_STRING)?,
        blaming_the(TINT_DISTANCE_FIELD, A_DISTANCE_MUST_BE_A_NUMBER),
        "the mistake a field whose value is a number invites from anybody used to a format \
         where everything is a string, and `far` rather than `'12'` because it is the spelling \
         somebody arrives at by describing what they want instead of measuring it. The sentence \
         is the one every other number on this declaration uses, deliberately: this field adds \
         a floor of its own and adds nothing else to the vocabulary, and a field that invented \
         its own wording for a mistake three other fields already name gives a mod author a \
         second dialect to learn"
    );
    Ok(())
}

#[test]
fn each_of_the_two_fields_stated_without_the_other_is_refused_naming_the_one_to_add() -> TestResult
{
    let colour_alone = TempDir::new()?;
    let distance_alone = TempDir::new()?;
    let no_distance = root_of(&colour_alone, &[(TINT_FIELD, A_COLOUR_THE_LOADER_ACCEPTS)])?;
    let no_colour = root_of(
        &distance_alone,
        &[(TINT_DISTANCE_FIELD, A_DISTANCE_THE_LOADER_ACCEPTS)],
    )?;

    assert_eq!(
        (refusal_of(&no_distance), refusal_of(&no_colour)),
        (
            blaming_the(TINT_DISTANCE_FIELD, A_DISTANCE_IS_REQUIRED_BESIDE_A_TINT),
            blaming_the(TINT_FIELD, A_TINT_IS_REQUIRED_BESIDE_A_DISTANCE),
        ),
        "a colour with no distance and a distance with no colour are each half a medium, and \
         the engine has no business completing either — the missing half would be an engine \
         constant standing in for content, which invariant 1 forbids. Two sentences rather \
         than one, and each blames the field that is **missing**, because that is the line the \
         author has to add: a single sentence covering both would have to be vague about which \
         of the two it means, and vague is the one thing a refusal read mid-edit cannot afford. \
         Each names the field the author already has, so nobody greps for a line their file \
         does not contain"
    );
    Ok(())
}
