//! Fixtures and readings shared by the Luau block-declaration suites.
//!
//! A declaration is a file a scripting host evaluates, so every fixture here
//! writes real Luau into a real directory and reads it back through the real
//! loader. The thing under test is precisely the reading of a directory of
//! chunks, and a mock of it would assert nothing.
//!
//! Three conventions run through all of it.
//!
//! **A block's texture key is never its own name.** Every scenario in the first
//! draft of this feature gave a block a texture equal to its name, and a loader
//! that read `name` into both fields was green throughout — the exact confusion
//! `BlockRegistry::texture_keys` warns about in its own doc comment. Every
//! fixture below states `example:amber` and `example:quartz`, and no fixture may
//! collapse the two.
//!
//! **An origin is compared by the name of the file it points at**, never by a
//! *written-down* path: a path renders with OS-specific separators, and an
//! assertion on one spelled out in a string literal would be a Windows-only or
//! Unix-only test.
//!
//! That rule now has one exception, and it is the reason [`declaration_label`]
//! and [`declarations_label`] exist. A refusal about a chunk has to point at the
//! file a person can open rather than at the bare name the scripting host was
//! given, and `contains("amber.luau")` is true of both — so it cannot separate
//! them and cannot falsify the requirement. The whole path is therefore compared
//! where that is the subject, but it is **built** with `Path::join` and rendered
//! the way the loader renders one, so no separator is written down and the
//! assertion stays portable.
//!
//! **No fixture here writes a texture file, and that is a property rather than
//! an omission.** A texture key is a reference the renderer will interpret and
//! never a path that has to exist, so every root below declares blocks whose keys
//! resolve to nothing on disk and registers all of them. It is stated here rather
//! than given a test of its own: the fixtures make it structural, and
//! `a_declaration_naming_a_texture_other_than_itself_registers_both_as_stated`
//! already asserts a key that could not be a file name for the block that
//! declared it. A second test through the same code path would witness nothing.
//!
//! **The loader owns the order a content root is read in.** It sorts by file
//! name, and one test asserts exactly that against a fixture whose file-name
//! order, block-name order and filesystem-listing order all disagree. Every
//! other reading here stays deliberately order-independent, so that a failure
//! anywhere else is about what it says it is rather than about a sort.

// Each test binary linking this module uses a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::Path;

use mc_core::block::source::{DefinitionFault, DefinitionSource, DefinitionSourceError};
use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::content::{Face, FaceTextures};
use mc_core::id::BlockName;
use mc_world::content::{LuauFileDefinitionSource, Printed};

/// The subdirectory of a content root that declarations live in.
///
/// Written here as well as in the loader because two scenarios ask a refusal to
/// name this directory by its path, and an expectation derived from the value
/// under test would agree with it whatever it became.
pub const BLOCKS_DIRECTORY: &str = "blocks";

/// The name the single-declaration fixtures give themselves.
pub const AMBER: &str = "example:amber";

/// The texture key those fixtures name.
///
/// Deliberately not [`AMBER`]. See the module note.
pub const QUARTZ: &str = "example:quartz";

/// The file a single-declaration fixture is written to.
pub const AMBER_FILE: &str = "amber.luau";

/// The residue the optional-field fixtures name.
///
/// A third id, distinct from both [`AMBER`] and [`QUARTZ`] for the same reason
/// those two are distinct from each other: a loader that wrote a block's own
/// name into its residue, or its texture key, must have somewhere to be wrong.
pub const ASH: &str = "example:ash";

/// The file the residue is declared in, where a fixture declares it at all.
pub const ASH_FILE: &str = "ash.luau";

/// A field stating `value` as Luau text.
///
/// Single-quoted, because a declaration is written into a Rust string literal
/// and doubling every escape makes a fixture unreadable at exactly the moment
/// somebody needs to read it.
#[must_use]
pub fn text_field(field: &str, value: &str) -> String {
    format!("{field} = '{value}'")
}

/// A field stating `value` verbatim — a boolean, a number, or anything else a
/// test wants to put where the loader expects something else.
#[must_use]
pub fn raw_field(field: &str, value: &str) -> String {
    format!("{field} = {value}")
}

/// A chunk returning a table of exactly `fields`.
#[must_use]
pub fn declaration_of(fields: &[String]) -> String {
    let mut chunk = String::from("return {\n");
    for field in fields {
        chunk.push('\t');
        chunk.push_str(field);
        chunk.push_str(",\n");
    }
    chunk.push_str("}\n");
    chunk
}

/// The three required fields of [`AMBER`], correctly stated.
#[must_use]
pub fn well_formed_amber() -> String {
    declaring(AMBER)
}

/// A well-formed declaration of `name`, textured [`QUARTZ`].
///
/// Every block a fixture declares carries the same texture key, which two blocks
/// may legitimately share. It is what keeps the name and the texture distinct in
/// every file this suite writes, including the ones whose subject is something
/// else entirely.
#[must_use]
pub fn declaring(name: &str) -> String {
    declaration_of(&[
        text_field("name", name),
        text_field("texture", QUARTZ),
        raw_field("solid", "true"),
    ])
}

/// A well-formed declaration of [`AMBER`] with `preamble` running first.
///
/// The shape every hostile fixture takes: a declaration that would register, and
/// one line in front of it that must stop it. A fixture built any other way
/// leaves it open whether the refusal was about the hostile line at all.
#[must_use]
pub fn amber_after(preamble: &str) -> String {
    format!("{preamble}{}", well_formed_amber())
}

/// A registry holding everything the content root at `root` declares.
///
/// # Errors
///
/// Returns an error if the root is refused.
pub fn registry_from(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root))?;
    Ok(registry)
}

/// The registry and the refusal that applying `root` produced.
///
/// # Errors
///
/// Returns an error if `root` was accepted, because every assertion made on the
/// refusal would then be vacuous.
pub fn refusal_from(root: &Path) -> Result<(BlockRegistry, RegistryError), Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    let refusal = registry
        .apply(&LuauFileDefinitionSource::new(root))
        .err()
        .ok_or("this content root must not be accepted, or the assertion below is vacuous")?;
    Ok((registry, refusal))
}

/// The fault reported for a root holding one badly-declared block.
///
/// # Errors
///
/// Returns an error if the root was accepted, or refused as something other than
/// a malformed declaration.
pub fn fault_from(root: &Path) -> Result<DefinitionFault, Box<dyn Error>> {
    let (_, refusal) = refusal_from(root)?;
    let RegistryError::Source(DefinitionSourceError::Malformed(fault)) = &refusal else {
        return Err(format!("expected a malformed-declaration refusal, got {refusal:?}").into());
    };
    Ok(fault.clone())
}

/// What a content root's refusal blamed, covering the outcome where it blamed
/// nothing.
///
/// **Total rather than fallible, for the reason [`registration_order_or_refusal`]
/// is.** A scenario about *which* field a refusal names has to be able to fail
/// on its own comparison when a loader accepts a root it should have refused —
/// propagating that with `?` ends the test before its assertion ever runs, and
/// a test that never reached its assertion has not shown it was checking the
/// right thing. Every arm is a different fact about the load, so a comparison
/// against one rejects the other two for free.
#[derive(Debug, PartialEq, Eq)]
pub enum Blamed {
    /// The root was accepted, so nothing was blamed for anything.
    NothingRefused,
    /// A declaration the loader would not accept, and what its fault said about
    /// where the fault is.
    Declaration(Attribution),
    /// Refused, but not as a malformed declaration, rendered as it renders
    /// itself.
    SomethingElse(String),
}

/// What the content root at `root` blamed, judged against `file`, together with
/// what the refusal said.
///
/// The cause is empty where there was no refusal, which is honest: a load that
/// was accepted said nothing about why it was not.
#[must_use]
pub fn judged(root: &Path, file: &str) -> (Blamed, String) {
    let mut registry = BlockRegistry::new();
    match registry.apply(&LuauFileDefinitionSource::new(root)) {
        Ok(()) => (Blamed::NothingRefused, String::new()),
        Err(RegistryError::Source(DefinitionSourceError::Malformed(fault))) => {
            let cause = fault.cause.clone();
            (Blamed::Declaration(attribution_of(&fault, file)), cause)
        }
        Err(other) => (Blamed::SomethingElse(other.to_string()), String::new()),
    }
}

/// What `root` blamed, for a test that reads nothing of the cause.
#[must_use]
pub fn blamed_by(root: &Path, file: &str) -> Blamed {
    judged(root, file).0
}

/// What a refusal managed to say about where it happened.
///
/// A record rather than three separate assertions, so one comparison reports
/// every field at once and a loader that got two of them right is not mistaken
/// for one that got them all right.
#[derive(Debug, PartialEq, Eq)]
pub struct Attribution {
    pub names_the_file: bool,
    pub block: Option<String>,
    pub field: Option<String>,
}

/// What `fault` says, judged against the file it should be pointing at.
#[must_use]
pub fn attribution_of(fault: &DefinitionFault, file: &str) -> Attribution {
    Attribution {
        names_the_file: fault.origin.as_str().contains(file),
        block: fault.block.clone(),
        field: fault.field.clone(),
    }
}

/// A refusal naming the file, the block as it named itself, and the field.
#[must_use]
pub fn blaming(block: &str, field: &str) -> Attribution {
    Attribution {
        names_the_file: true,
        block: Some(block.to_owned()),
        field: Some(field.to_owned()),
    }
}

/// A refusal naming the file and the block as it named itself, with no single
/// field to send its author to.
///
/// The shape a refusal takes when a declaration is wrong *as a whole* rather
/// than in one place — it holds more fields than the loader will read, say. The
/// block is still named because the loader reads the name before it checks
/// anything, so it has one to quote back even when it has no field to blame.
#[must_use]
pub fn blaming_the_declaration(block: &str) -> Attribution {
    Attribution {
        names_the_file: true,
        block: Some(block.to_owned()),
        field: None,
    }
}

/// A refusal naming the file and the field, with no block to quote back.
#[must_use]
pub fn blaming_field_alone(field: &str) -> Attribution {
    Attribution {
        names_the_file: true,
        block: None,
        field: Some(field.to_owned()),
    }
}

/// A refusal naming the file and nothing else, there being neither a block nor a
/// field to name.
#[must_use]
pub fn naming_the_file_alone() -> Attribution {
    Attribution {
        names_the_file: true,
        block: None,
        field: None,
    }
}

/// What `registry` holds for `name`, as one comparable line.
///
/// Rendered rather than tupled so that a mismatch reads as the declaration a mod
/// author wrote beside the one the loader built.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced id or the registry does not
/// hold it.
pub fn registered(registry: &BlockRegistry, name: &str) -> Result<String, Box<dyn Error>> {
    let definition = registry.resolve(&BlockName::parse(name)?)?;
    Ok(format!(
        "textured {}, solid {}",
        textured(&definition.textures),
        definition.is_solid
    ))
}

/// The six facing words, in the order a refusal lists them.
///
/// Written out here rather than read from [`Face::ALL`], for the reason every
/// other expectation in this suite is written out: a list derived from the value
/// under test agrees with whatever that value becomes.
pub const SIX_FACINGS: [&str; 6] = ["up", "down", "north", "south", "east", "west"];

/// A `texture` field stating a table, each word holding the key written against
/// it.
///
/// Takes pairs rather than six keys, because most of what this builds is a table
/// that is **wrong** — three facings, none, one spelled `top` — and a fixture
/// that could only express a well-formed one could not write the refusals.
#[must_use]
pub fn facing_table(facings: &[(&str, &str)]) -> String {
    let stated: String = facings
        .iter()
        .map(|(word, key)| format!("\t\t{word} = '{key}',\n"))
        .collect();
    format!("texture = {{\n{stated}\t}}")
}

/// How a block's six facing keys read in a comparison.
///
/// One key where all six agree, and the six words with their keys where they do
/// not. **The collapse is deliberate and it is what keeps every fixture that
/// states one string reading exactly as it did before a block had six facings**;
/// the expansion is what stops a set that lost five of its six from rendering as
/// the one it kept.
#[must_use]
pub fn textured(textures: &FaceTextures) -> String {
    let uniform = textures.at(Face::Up);
    if Face::ALL.iter().all(|face| textures.at(*face) == uniform) {
        return uniform.as_str().to_owned();
    }
    Face::ALL
        .iter()
        .map(|face| format!("{} {}", face.as_str(), textures.at(*face).as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Which key `registry` holds against each of `name`'s six facings, in
/// [`Face::ALL`] order.
///
/// One line per facing rather than a map, so a mismatch reads as the table a mod
/// author wrote — and so that a loader which resolved five facings correctly is
/// not mistaken for one that resolved them all correctly.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced id or the registry does not
/// hold it.
pub fn facings_of(registry: &BlockRegistry, name: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let definition = registry.resolve(&BlockName::parse(name)?)?;
    Ok(Face::ALL
        .iter()
        .map(|face| {
            format!(
                "{} = {}",
                face.as_str(),
                definition.textures.at(*face).as_str()
            )
        })
        .collect())
}

/// What a declaration said about how the world may be changed around a block:
/// the three fields it is allowed to leave out.
///
/// A record rather than three separate readings, so one comparison reports all
/// three at once and a loader that resolved two of them correctly is not
/// mistaken for one that resolved them all correctly. The residue is carried as
/// text because the assertion is about *which* id was retained, and a `BlockId`
/// would have had to be resolved — which is the one thing a residue must not be.
#[derive(Debug, PartialEq, Eq)]
pub struct Behaviour {
    pub replaceable: bool,
    pub breakable: bool,
    pub breaks_into: Option<String>,
}

/// What a block that says nothing about any of the three means.
///
/// Written out here rather than derived from a definition the loader built, so
/// that a default resolved differently has something to disagree with.
#[must_use]
pub fn the_documented_defaults() -> Behaviour {
    Behaviour {
        replaceable: false,
        breakable: true,
        breaks_into: None,
    }
}

/// What `registry` holds for `name` in the three optional fields.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced id or the registry does not
/// hold it.
pub fn behaviour_of(registry: &BlockRegistry, name: &str) -> Result<Behaviour, Box<dyn Error>> {
    let definition = registry.resolve(&BlockName::parse(name)?)?;
    Ok(Behaviour {
        replaceable: definition.replaceable,
        breakable: definition.breakable,
        breaks_into: definition
            .breaks_into
            .as_ref()
            .map(|residue| residue.as_str().to_owned()),
    })
}

/// Which of `needles` a refusal's cause mentions, in the order it mentions
/// them.
///
/// Order by position rather than a substring check per name, because two of
/// this suite's scenarios are about the order a refusal lists fields in and a
/// per-name check cannot see one. A needle the cause never mentions is left
/// out, so the answer says both which and in what order.
#[must_use]
pub fn named_in_order<'a>(cause: &str, needles: &[&'a str]) -> Vec<&'a str> {
    let mut found: Vec<(usize, &'a str)> = needles
        .iter()
        .filter_map(|needle| cause.find(needle).map(|at| (at, *needle)))
        .collect();
    found.sort_by_key(|&(at, _)| at);
    found.into_iter().map(|(_, needle)| needle).collect()
}

/// Every texture key `registry` holds, sorted.
#[must_use]
pub fn texture_keys(registry: &BlockRegistry) -> Vec<String> {
    registry
        .texture_keys()
        .iter()
        .map(|key| key.as_str().to_owned())
        .collect()
}

/// Every name a source yields, sorted.
///
/// Sorted deliberately, even though the loader now owns the read order: the
/// callers of this reading are about *which* names a root declares, and folding
/// the order into them would make an unrelated failure report as an ordering
/// one. The order has a test of its own — see [`registration_order_from`].
///
/// # Errors
///
/// Returns an error if the source refuses any declaration.
pub fn names_yielded(source: &LuauFileDefinitionSource) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = Vec::new();
    for yielded in source.definitions() {
        names.push(yielded?.name.as_str().to_owned());
    }
    names.sort();
    Ok(names)
}

/// What content printed while `root` was read, and whether the root registered.
///
/// Both halves are needed wherever the assertion is that **nothing** was
/// printed: a loader that never evaluated a chunk at all prints nothing too, and
/// an emptiness asserted on its own cannot tell the two apart.
///
/// **The record is one value and not a list beside a count**, which is what
/// makes every "nothing was printed" assertion in this suite also reject a
/// record the host had stopped keeping. A list on its own cannot say which of
/// the two it is, and a caller reading the list can only fail to ask.
///
/// # Errors
///
/// Returns an error only if the fixture path cannot be turned into a source,
/// which it cannot today — the reading itself is reported rather than
/// propagated, because a root that is refused has still been read.
pub fn read_reporting_print(root: &Path) -> Result<(Printed, bool), Box<dyn Error>> {
    let source = LuauFileDefinitionSource::new(root);
    let mut registry = BlockRegistry::new();
    let registered = registry.apply(&source).is_ok();
    Ok((source.printed(), registered))
}

/// The declarations directory of `root`, rendered the way a refusal renders a
/// path.
///
/// Built with `Path::join` rather than written out, so nothing here spells a
/// separator and the expectation reads the same on either kind of system.
#[must_use]
pub fn declarations_label(root: &Path) -> String {
    root.join(BLOCKS_DIRECTORY).display().to_string()
}

/// One entry of `root`'s declarations directory, rendered the same way.
#[must_use]
pub fn declaration_label(root: &Path, entry: &str) -> String {
    root.join(BLOCKS_DIRECTORY)
        .join(entry)
        .display()
        .to_string()
}

/// What became of a content root, as one value rather than as a chain of
/// questions.
///
/// An enumerated verdict, because the alternative — asking whether a refusal
/// happened and then asking what it said — cannot tell a root that was accepted
/// from one refused for a reason nobody looked at. Every arm is a different fact
/// about the load, and a comparison against one of them rejects the other three
/// for free.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The root was accepted and nothing was refused at all.
    NothingRefused,
    /// A path the loader could not list or read, as the refusal named it.
    Unreadable { path: String },
    /// A declaration the loader would not accept, as the refusal named it,
    /// together with the block it managed to attribute the fault to.
    Malformed { path: String, block: Option<String> },
    /// The registry refused the batch on its own terms, rendered as it renders
    /// itself.
    RefusedByTheRegistry(String),
}

/// What became of the content root at `root`.
#[must_use]
pub fn refusal_of(root: &Path) -> Refusal {
    refusal_and_cause(root).0
}

/// What became of the content root at `root`, together with what the refusal
/// gave as its reason.
///
/// The cause travels with the verdict because the scenarios about bounds turn
/// on **which** refusal won rather than on whether one happened. A root holding
/// one file too many and one broken file is refused either way; only the reason
/// separates a loader that counted before it read from one that read first and
/// counted after, and the verdict alone cannot see that. The cause is empty
/// where nothing was refused, which is honest — a load that was accepted said
/// nothing about why it was not.
#[must_use]
pub fn refusal_and_cause(root: &Path) -> (Refusal, String) {
    let mut registry = BlockRegistry::new();
    match registry.apply(&LuauFileDefinitionSource::new(root)) {
        Ok(()) => (Refusal::NothingRefused, String::new()),
        Err(RegistryError::Source(DefinitionSourceError::Unreadable { origin, cause })) => (
            Refusal::Unreadable {
                path: origin.as_str().to_owned(),
            },
            cause,
        ),
        Err(RegistryError::Source(DefinitionSourceError::Malformed(fault))) => (
            Refusal::Malformed {
                path: fault.origin.as_str().to_owned(),
                block: fault.block,
            },
            fault.cause,
        ),
        Err(other) => (
            Refusal::RefusedByTheRegistry(other.to_string()),
            String::new(),
        ),
    }
}

/// Which of `needles` a refusal's cause mentions, in a fixed order.
///
/// Sorted rather than left in the order the cause names them: these scenarios
/// ask a refusal to *state* an observed quantity and a bound, and say nothing
/// about the sentence it builds from the two. Comparing a sorted list rejects
/// both halves of the failure at once — a refusal missing one of the numbers,
/// and a refusal naming something it was told not to name — while an assertion
/// per needle can only report one at a time.
#[must_use]
pub fn mentioning(cause: &str, needles: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = needles
        .iter()
        .filter(|needle| cause.contains(**needle))
        .map(|needle| (*needle).to_owned())
        .collect();
    found.sort();
    found
}

/// Every name `registry` holds, in the order it registered them.
///
/// Read back through the dense runtime ids, which are assigned as definitions
/// arrive — so this is registration order itself and not a re-sort of it.
///
/// # Errors
///
/// Returns an error if an id the registry counted does not resolve.
pub fn names_in_registration_order(
    registry: &BlockRegistry,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = Vec::with_capacity(registry.registered_count());
    for position in 0..registry.registered_count() {
        let id = BlockId::from_raw(u32::try_from(position)?);
        names.push(registry.definition(id)?.name.as_str().to_owned());
    }
    Ok(names)
}

/// Every name the content root at `root` registers, in registration order — or
/// the refusal that stopped it, rendered.
///
/// **Total rather than fallible, and that is the whole point of it.** A scenario
/// about which entries are declarations has to be able to fail on its own
/// assertion when a loader refuses a root it should have read; propagating the
/// refusal with `?` ends the test before the assertion runs, and a test that
/// never reached its assertion has checked nothing. Both outcomes are values
/// here, so one comparison judges either.
pub fn registration_order_or_refusal(root: &Path) -> Result<Vec<String>, String> {
    let registry = registry_from(root).map_err(|refusal| refusal.to_string())?;
    names_in_registration_order(&registry).map_err(|broken| broken.to_string())
}
