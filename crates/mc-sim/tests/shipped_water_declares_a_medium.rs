//! What the four blocks this repository ships say about the volumes they fill.
//!
//! **Two readings of one claim, and neither can see what the other sees.** The
//! registry knows what a block *registers as*, and cannot tell a field an author
//! left out from one an author wrote the default into — `swimmable = false` on
//! dirt registers exactly as dirt's silence does. The declaration files know what
//! an author *wrote*, and know nothing about what the loader made of it. Both
//! halves are claimed — water declaring a medium, and the three solid blocks
//! declaring neither field — so both are read.
//!
//! **Each list is compared whole and in order**, never filtered and never looked
//! up name by name. A comparison that walked the expectations and asked whether
//! the registry held each one cannot see a fifth block that quietly declares
//! itself swimmable, and a comparison that skipped what it could not rank cannot
//! see a missing one. Read whole, a missing block, an extra block and a
//! reordering are three distinct failures.
//!
//! Registration order is the loader's own: it sorts by file name, so the four
//! arrive as `dirt`, `grass`, `stone`, `water`.

mod support;

use std::error::Error;
use std::fs;

use mc_core::block::BlockDefinition;

use support::sea;
use support::{DIRT, GRASS, STONE, TestResult, WATER, content_registry, repository_root};

/// The file each shipped block is declared in, in the order the loader reads
/// them.
const DECLARATION_FILES: [(&str, &str); 4] = [
    (DIRT, "dirt.luau"),
    (GRASS, "grass.luau"),
    (STONE, "stone.luau"),
    (WATER, "water.luau"),
];

/// The two medium field names a declaration's own text is read for.
const SWIMMABLE: &str = "swimmable";
const MOVE_RESISTANCE: &str = "move_resistance";

/// What a registered definition says about the volume it fills.
///
/// **Three answers and not two.** The third carries the values it found, so a
/// block that is swimmable and resists nothing, or resistant and impossible to
/// swim in, is *named* rather than folded into "not what was expected" — and a
/// resistance that is negative or absurd arrives as a figure a reader can act on.
#[derive(Debug, PartialEq, Eq)]
enum Registers {
    /// Nothing to swim in and nothing to resist: what a declaration stating
    /// neither field means.
    NeitherAMediumNorAResistance,
    /// A volume a player can hold itself up in, which also resists more than
    /// nothing.
    SwimmableAndResistant,
    /// Anything else, rendered.
    Otherwise(String),
}

/// What a declaration file states about the volume its block fills.
///
/// The same three shapes, asked of the text an author wrote rather than of what
/// the loader made of it.
#[derive(Debug, PartialEq, Eq)]
enum States {
    /// The declaration names neither field, so both of its answers are the
    /// constants an absence means.
    NeitherField,
    /// The declaration names both.
    BothFields,
    /// It names one of them and not the other, said which way round.
    Otherwise(String),
}

/// How a definition registers.
fn registers(definition: &BlockDefinition) -> Registers {
    match (definition.swimmable, definition.move_resistance > 0.0) {
        (false, false) => Registers::NeitherAMediumNorAResistance,
        (true, true) => Registers::SwimmableAndResistant,
        _ => Registers::Otherwise(format!(
            "swimmable = {}, move_resistance = {}",
            definition.swimmable, definition.move_resistance
        )),
    }
}

/// What each shipped block registers as, read out of the registry in the order
/// the loader registered them.
fn what_the_shipped_blocks_register() -> Result<Vec<(String, Registers)>, Box<dyn Error>> {
    Ok(content_registry()?
        .definitions()
        .map(|declared| (declared.name.as_str().to_owned(), registers(declared)))
        .collect())
}

/// What each shipped declaration file states, read out of its own text.
fn what_the_shipped_declarations_state() -> Result<Vec<(String, States)>, Box<dyn Error>> {
    let blocks = repository_root()?
        .join("content")
        .join("base")
        .join("blocks");
    let mut stated = Vec::with_capacity(DECLARATION_FILES.len());
    for (name, file) in DECLARATION_FILES {
        let declaration = fs::read_to_string(blocks.join(file))?;
        stated.push((name.to_owned(), states(&declaration)));
    }
    Ok(stated)
}

/// Which of the two fields a declaration's text assigns.
///
/// The assignment and not the bare name, so that a comment discussing a field —
/// which every shipped declaration's header does at length about the fields it
/// states — is not read as a declaration of it.
fn states(declaration: &str) -> States {
    let assigns = |field: &str| declaration.contains(&format!("{field} ="));
    match (assigns(SWIMMABLE), assigns(MOVE_RESISTANCE)) {
        (false, false) => States::NeitherField,
        (true, true) => States::BothFields,
        (swimmable, _) => States::Otherwise(format!(
            "it names `{}` and not `{}`",
            if swimmable {
                SWIMMABLE
            } else {
                MOVE_RESISTANCE
            },
            if swimmable {
                MOVE_RESISTANCE
            } else {
                SWIMMABLE
            }
        )),
    }
}

/// One expected registration, spelled the way the comparison reads it.
fn registering(name: &str, answer: Registers) -> (String, Registers) {
    (name.to_owned(), answer)
}

/// One expected declaration, likewise.
fn stating(name: &str, answer: States) -> (String, States) {
    (name.to_owned(), answer)
}

#[test]
fn the_shipped_sea_is_the_only_block_that_registers_as_something_to_swim_in() -> TestResult {
    assert_eq!(
        what_the_shipped_blocks_register()?,
        vec![
            registering(DIRT, Registers::NeitherAMediumNorAResistance),
            registering(GRASS, Registers::NeitherAMediumNorAResistance),
            registering(STONE, Registers::NeitherAMediumNorAResistance),
            registering(WATER, Registers::SwimmableAndResistant),
        ],
        "the shipped content registers four blocks: water as a volume a player can hold itself up \
         in and which resists more than nothing, and the three that make the ground as neither. \
         The whole list is compared in registration order, so a block that has stopped declaring \
         a medium, one that has started, a fifth block and a reordering are four different \
         failures rather than one"
    );
    Ok(())
}

#[test]
fn only_the_sea_declaration_names_either_medium_field_at_all() -> TestResult {
    assert_eq!(
        what_the_shipped_declarations_state()?,
        vec![
            stating(DIRT, States::NeitherField),
            stating(GRASS, States::NeitherField),
            stating(STONE, States::NeitherField),
            stating(WATER, States::BothFields),
        ],
        "the ground blocks declare *neither* medium field, which the registry cannot \
         report: `swimmable = false` written into dirt registers exactly as dirt's silence does. \
         So the declarations are read as text. Water is the control that this reading can see a \
         field at all — without it, a scan that had come to find nothing anywhere would report \
         three silent ground blocks forever"
    );
    Ok(())
}

/// What the shipped `base:water` is required to declare about the volume it
/// fills, written out by hand.
///
/// **Stated rather than derived, and that is now the specification's own
/// requirement rather than this file's caution.** Play has judged these numbers,
/// so they are no longer values a scenario should leave free: the observable
/// rates the game promises are stated absolutely, and these three are what
/// produce them. Deriving any of them from the registry would compare a value to
/// itself and pass forever.
///
/// **The hole this closes was measured.** Changing the resistance from `1.6` to
/// `2.0` — another value the old admissible window admitted — reddened exactly
/// one test in 702, `terrain_goldens`, because the scripted walk wades through
/// this sea. A commit that changed the number and re-minted the four golden
/// directories in the same breath was therefore reported by nothing at all.
/// Nothing here looks at a golden.
const SHIPPED_SWIMMABLE: bool = true;
const SHIPPED_RESISTANCE: f32 = 0.5;
const SHIPPED_ASCENT: f32 = 3.5;

#[test]
fn the_shipped_water_declares_the_medium_the_stated_rates_are_derived_from() -> TestResult {
    let registry = content_registry()?;
    let declared = registry.resolve(&support::block_name(WATER)?)?;

    assert_eq!(
        (
            declared.swimmable,
            sea::declared_resistance(&registry)?.to_bits(),
            declared.swim_ascent.to_bits()
        ),
        (
            SHIPPED_SWIMMABLE,
            SHIPPED_RESISTANCE.to_bits(),
            SHIPPED_ASCENT.to_bits()
        ),
        "the sea's declared medium moved: it reads swimmable = {}, move_resistance = {}, \
         swim_ascent = {} where the specification states {SHIPPED_SWIMMABLE}, \
         {SHIPPED_RESISTANCE} and {SHIPPED_ASCENT}. Do not simply re-mint these. What a change \
         owes is three things. **One**: re-derive the observable rates the game promises — a \
         sink of `(1/60)·[60 − 2(1 − (2/3)^60)]` blocks in a second, a rise of \
         `(swim_ascent − 0.5)/(1 + move_resistance)` blocks per second, and a swim of \
         `4.5/(1 + move_resistance)` — and move the scenarios that state them. **Two**: \
         re-shoot all four golden directories by the procedure in \
         `docs/technical/rendering.md`, because the scripted walk wades through this sea and \
         its poses move with these figures. **Three**: re-derive the player-facing figures in \
         `docs/user/gameplay.md`, which state the sink, the swim fraction and how long \
         reaching the surface takes",
        declared.swimmable,
        declared.move_resistance,
        declared.swim_ascent
    );
    Ok(())
}
