//! The catalogue of shipped content roots a scenario asks for by name, and the
//! declaration text each one rewrites.
//!
//! **The seam is who is asking.** [`super::ContentRoot`] is what a fixture may
//! *do* to a root it already holds — add a declaration, take one out, empty a
//! directory, say where it sits. This module is the catalogue of roots a
//! scenario names outright: the shipped tree with its sea declaring a degree,
//! with a block renamed, with an element filled in its own outline colour, with
//! a tint pair written or taken away. Every one of them is
//! [`super::shipped_copy`] plus one stated edit, and every edit is to the **text
//! inside** a declaration rather than to which files are there.
//!
//! **A root is still always copied, never edited in place**, and the parent's
//! header says why — the rule is the module's, not this file's, and nothing here
//! relaxes it.
//!
//! They were one file until it crossed the 600-line cap. The cap forced the
//! question; it did not answer it. What a fixture can do to a directory and
//! which named states of the shipped tree scenarios ask for are two
//! responsibilities, and the day a new root joins the catalogue is not the day
//! `ContentRoot` grows a method.

use std::error::Error;
use std::fs;

use super::{BLOCK_DIRECTORY, ContentRoot, HUD_DIRECTORY, shipped_copy};
use crate::support::content_root;

/// The file the shipped sea is declared in.
pub const SEA_DECLARATION: &str = "water.luau";

/// The line the degree is written directly beneath, which is the field that puts
/// water on screen at all.
const DRAWN: &str = "\tdrawn = true,\n";

/// The shipped content root copied with its sea declaring `degree`.
///
/// **The shipped declaration with one line added, read off disk rather than
/// restated here.** What this builds is the shipped root as it stands *plus* the
/// one field, so every reading over it is about the shipped world, the shipped
/// art, the shipped physics and the shipped strata — and the day the shipped
/// root declares the degree itself, the only difference left is the degree's
/// value. A declaration written out in this module instead would drift from the
/// shipped one silently, and a reading about "the sea" would be about a sea
/// nobody ships.
///
/// # Errors
///
/// Returns an error if the shipped declaration cannot be read or no longer
/// carries the line the degree is written beneath, or if the copy fails.
pub fn shipped_with_the_sea_declaring(degree: f32) -> Result<ContentRoot, Box<dyn Error>> {
    let at = content_root()?.join(BLOCK_DIRECTORY).join(SEA_DECLARATION);
    let shipped = fs::read_to_string(&at)?;
    if !shipped.contains(DRAWN) {
        return Err(format!(
            "`{}` no longer states `{}` on a line of its own, so this fixture has nowhere to write \
             the degree it is about. It has to be added to the shipped declaration rather than to \
             one written here, or every reading over it is about a sea nobody ships",
            at.display(),
            DRAWN.trim()
        )
        .into());
    }
    let stated = shipped.replace(DRAWN, &format!("{DRAWN}\topacity = {degree:?},\n"));
    shipped_copy()?
        .not_declaring_blocks(&[SEA_DECLARATION])?
        .declaring_block(SEA_DECLARATION, &stated)
}

/// The file the shipped sea is declared in, and the one the surface layer is.
pub const SEA_FILE: &str = SEA_DECLARATION;
pub const SURFACE_FILE: &str = "grass.luau";

impl ContentRoot {
    /// This root with the block declared in `file_name` stating `tint` reaching
    /// its full strength at that distance, or stating no tint at all.
    ///
    /// **The declaration is the shipped one with its tint pair rewritten, read
    /// off disk rather than restated here** — the rule
    /// [`shipped_with_the_sea_declaring`] follows, and for the same reason:
    /// every reading over the result is about the shipped world, the shipped
    /// art, the shipped physics and the shipped strata, with the tint pair as
    /// the only difference.
    ///
    /// **Rewriting tolerates a declaration that stated no tint to begin with,
    /// which is a deliberate exception to this module's "removing what was never
    /// there is a failure" rule.** What a reading here needs is a root in a
    /// stated *state*, and the shipped sea reaches "declares no tint" by two
    /// routes across this spec's own commits — it states none today and states
    /// one once the sea is declared. A helper that refused the first would break
    /// on the commit that declares it and a helper that refused the second would
    /// break on every commit before it.
    ///
    /// # Errors
    ///
    /// Returns an error if the declaration cannot be read or states no name of
    /// its own for the pair to be written beneath, or if the write fails.
    pub fn whose_block_declares(
        self,
        file_name: &str,
        tint: Option<([u8; 3], f32)>,
    ) -> Result<Self, Box<dyn Error>> {
        let at = self.path().join(BLOCK_DIRECTORY).join(file_name);
        let stated = fs::read_to_string(&at)?;
        let pair = tint.map_or_else(String::new, |([red, green, blue], distance)| {
            format!(
                "\ttint = \"#{red:02X}{green:02X}{blue:02X}\",\n\ttint_distance = {distance:?},\n"
            )
        });
        let kept: Vec<&str> = stated
            .lines()
            .filter(|line| !is_a_tint_line(line))
            .collect();
        let named = kept.iter().any(|line| names_the_block(line));
        let written: String = kept.iter().map(|line| restated(line, &pair)).collect();
        if !named {
            return Err(format!(
                "`{BLOCK_DIRECTORY}/{file_name}` states no `{NAME_FIELD}` line of its own, so \
                 this fixture has nowhere to write the tint pair it is about. It has to be added \
                 to the shipped declaration rather than to one written here, or every reading \
                 over it is about a block nobody ships"
            )
            .into());
        }
        fs::write(&at, written)?;
        Ok(self)
    }
}

/// The field every block declaration opens with, which the tint pair is written
/// directly beneath.
const NAME_FIELD: &str = "name = ";

/// Whether `line` is the declaration's own name field.
///
/// Matched past the indent and from the start of the line, so the word inside
/// one of these files' prose comments cannot be hit.
fn names_the_block(line: &str) -> bool {
    line.trim_start().starts_with(NAME_FIELD)
}

/// `line`, with `pair` written on the line beneath it where it is the name
/// field.
fn restated(line: &str, pair: &str) -> String {
    let beneath = if names_the_block(line) { pair } else { "" };
    format!("{line}\n{beneath}")
}

/// Whether `line` states one of the two tint fields.
///
/// Matched from the start of the line past its indent, so the word inside one of
/// these files' prose comments cannot be hit.
fn is_a_tint_line(line: &str) -> bool {
    let stated = line.trim_start();
    stated.starts_with("tint = ") || stated.starts_with("tint_distance = ")
}

/// The shipped content root copied with the named HUD declarations removed.
///
/// # Errors
///
/// Returns an error if the copy fails, or if a named declaration was not there
/// to remove — see this module's header for why that is a failure rather than
/// nothing happening.
pub fn shipped_without(declarations: &[&str]) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    for file_name in declarations {
        let declared = copied.path().join(HUD_DIRECTORY).join(file_name);
        if !declared.is_file() {
            return Err(format!(
                "this fixture has to remove `{HUD_DIRECTORY}/{file_name}` from a copy of the \
                 shipped content root, but the shipped root does not declare it. What it would \
                 build is a root that never had a crosshair rather than one whose crosshair was \
                 taken away, and the two are not the same claim"
            )
            .into());
        }
        fs::remove_file(&declared)?;
    }
    Ok(copied)
}

/// The shipped content root copied with one block definition file renamed.
///
/// **The declaration inside is untouched; only its file name moves.** Blocks are
/// registered in file-name sorted order and a client holds the first solid block
/// in that order, so renaming one file is the smallest edit that changes which
/// block a run holds — and it changes nothing else: the same four blocks are
/// registered, the same world generates, and the same texture keys resolve to
/// the same layers. Deleting a definition instead would change the world as
/// well as the held block, and two frames differing for two reasons say nothing
/// about either.
///
/// # Errors
///
/// Returns an error if the copy or the rename fails, or if `from` was not there
/// to rename — a root that never declared it is not a root whose declaration
/// moved.
pub fn shipped_renaming_block(from: &str, to: &str) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    let blocks = copied.path().join(BLOCK_DIRECTORY);
    let declared = blocks.join(from);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to rename `{BLOCK_DIRECTORY}/{from}` inside a copy of the shipped \
             content root, but the shipped root does not declare it. What it would build is a \
             root that registers the same blocks in the same order, and the two frames a \
             scenario compares would then hold the same block for a reason nothing states"
        )
        .into());
    }
    fs::rename(&declared, blocks.join(to))?;
    Ok(copied)
}

/// The shipped content root copied with several block declarations renamed, in
/// the order given.
///
/// **More than one, because which block a client holds is the *first solid* one
/// in file-name order** — so moving one declaration out of the way only reaches
/// the second, and reaching the third needs two moved. A scenario needing two
/// blocks whose textures share no colour cannot always take the first two it is
/// offered.
///
/// # Errors
///
/// Returns an error if any named declaration is not there to rename, for the
/// reason [`shipped_renaming_block`] gives.
pub fn shipped_renaming_blocks(renames: &[(&str, &str)]) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    let blocks = copied.path().join(BLOCK_DIRECTORY);
    for (from, to) in renames {
        let declared = blocks.join(from);
        if !declared.is_file() {
            return Err(format!(
                "this fixture has to rename `{BLOCK_DIRECTORY}/{from}` inside a copy of the \
                 shipped content root, but the shipped root does not declare it. What it would \
                 build is a root that registers the same blocks in the same order, and the two \
                 frames a scenario compares would then hold the same block for a reason nothing \
                 states"
            )
            .into());
        }
        fs::rename(&declared, blocks.join(to))?;
    }
    Ok(copied)
}

/// The shipped content root copied with one more declaration written into
/// `hud/`.
///
/// # Errors
///
/// Returns an error if the copy or the write fails.
pub fn shipped_with(file_name: &str, declaration: &str) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    let declared = copied.path().join(HUD_DIRECTORY);
    fs::create_dir_all(&declared)?;
    fs::write(declared.join(file_name), declaration)?;
    Ok(copied)
}

/// The shipped content root copied with the named HUD declarations restating
/// their `outline` colour as their fill `color`.
///
/// **Both colours come out of the shipped declaration, so nothing here states a
/// colour of its own.** What this builds is the frame a negative control needs: a
/// crosshair whose fill pixels really are drawn, and drawn in the colour the same
/// declaration reserves for its outline. A prediction that accepted it would be
/// accepting "something was painted here" in place of "the declared colour was".
///
/// # Errors
///
/// Returns an error if the copy, the read or the write fails, or if a named
/// declaration states no `color` or no `outline` to move — a root that never
/// declared one is not a root whose fill colour changed.
pub fn shipped_filling_with_the_outline_color(
    declarations: &[&str],
) -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    for file_name in declarations {
        let declared = copied.path().join(HUD_DIRECTORY).join(file_name);
        let stated = fs::read_to_string(&declared)?;
        let filled = line_of(&stated, COLOR_FIELD, file_name)?;
        let outlined = value_of(&stated, OUTLINE_FIELD, file_name)?;
        if filled.ends_with(&outlined) {
            return Err(format!(
                "`{HUD_DIRECTORY}/{file_name}` already fills with the colour it outlines with, so \
                 restating it changes nothing and the control below would be about the shipped \
                 declaration rather than about a fill in the wrong colour"
            )
            .into());
        }
        let restated = stated.replace(&filled, &format!("{COLOR_FIELD} = {outlined}"));
        fs::write(&declared, restated)?;
    }
    Ok(copied)
}

/// The field a declaration states its fill colour in, and the one it states its
/// contrast outline in, as a declaration spells them.
const COLOR_FIELD: &str = "color";
const OUTLINE_FIELD: &str = "outline";

/// The whole `field = value` line `stated` holds, matched from the start of the
/// line so the word inside one of these files' prose comments cannot be hit.
///
/// # Errors
///
/// Returns an error naming the field and the file when the declaration does not
/// state it.
fn line_of(stated: &str, field: &str, file_name: &str) -> Result<String, Box<dyn Error>> {
    let opening = format!("{field} = ");
    stated
        .lines()
        .find(|line| line.starts_with(&opening))
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "this fixture has to restate `{HUD_DIRECTORY}/{file_name}`'s `{field}`, but that \
                 declaration does not state it. What it would build is a root the control below \
                 was never going to be about"
            )
            .into()
        })
}

/// What `stated` states `field` as, quotes included.
///
/// # Errors
///
/// Returns an error naming the field and the file when the declaration does not
/// state it.
fn value_of(stated: &str, field: &str, file_name: &str) -> Result<String, Box<dyn Error>> {
    let line = line_of(stated, field, file_name)?;
    Ok(line
        .split_once(" = ")
        .map_or(line.clone(), |(_, value)| value.to_owned()))
}
