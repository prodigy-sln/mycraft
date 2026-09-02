//! What a block was declared to be when a save was written, folded into the two
//! numbers a save stores against each name.
//!
//! Split out of [`super::format`], which answers what a save is *made of*: a
//! preamble, a table of names and a world. This answers a different question —
//! given a definition the registry holds *now*, what does the save record say it
//! was — and it changes for its own reasons, whenever one of the two canonical
//! field lists grows.
//!
//! The two lists and the two revision bytes are the whole of it, and the reason
//! they are two rather than one is argued at [`BEHAVIOUR_REVISION`].

use mc_core::block::BlockDefinition;
use mc_core::content::Face;
use mc_core::hash::fnv_1a_64;
use mc_core::id::BlockName;
use serde::Serialize;

use super::format::DefinitionHash;

/// Which revision of each canonical field list below a hash was folded over.
///
/// Its own leading byte, so that adding a field to one of them is a deliberate
/// act that says so in the value rather than silently reinterpreting every hash
/// already stored.
///
/// **One number per list, never one shared between them, and do not unify them.**
/// Two constants where there could be one reads as duplication, which is why the
/// reason is here and not only in the design notes. The two lists grow for
/// unrelated reasons, and a shared byte would move every block's *behaviour* hash
/// in every save in existence the moment the appearance list gained a field.
///
/// Measured when the appearance list grew to six keys, **and it is a reading of
/// that tree rather than of this one**: the committed pre-spec save reported all
/// four of its blocks `changed` under a shared byte and `retextured` under this
/// arrangement. Those are not two shades of one answer — a non-empty `changed`
/// **refuses the load** under
/// [`Acceptance::OnlyUnchangedBlocks`](super::table::Acceptance), and
/// `retextured` never refuses at all. Unifying these turns every world saved
/// before a retexture into a refused one.
///
/// **Do not re-read that measurement against today's answer and conclude the
/// split bought nothing.** The behaviour byte has since moved on its own account,
/// so that same save now reports all four `changed` under *either* arrangement —
/// the two agree today because a behaviour field really was added, which is the
/// one circumstance in which they are supposed to agree. What the split bought is
/// that the appearance list's two growths cost no player anything, and no later
/// reading can recover that from a tree where both bytes have moved.
///
/// Both lists have now grown, and for unrelated reasons each time: the
/// appearance list gained five keys and then `drawn` and `occludes`, and the
/// behaviour list gained `targetable`, then the two medium fields, and then the
/// ascent a medium carries a swimmer at. Each move is in that list's own byte
/// and in no other, which is the arrangement working rather than an exception to
/// it. `docs/technical/world-format.md` carries the numbers.
///
/// **They were equal at 3 until this move, and that equality was the coincidence
/// of counting it looked like.** They had arrived there by different routes — two
/// growths each, none of them shared — and this change moved one of them alone,
/// which is what the arrangement exists to allow. Unified on the strength of that
/// equality, every save in existence would now report every block as retextured
/// over a number no still frame can show.
///
/// **The behaviour byte's move is the expensive one, and it has now been paid
/// three times.** Every block of every save written before a move reports as
/// `changed` on its next load, so every player is told the blocks they built with
/// behave differently. That is survivable only because such a save loads and
/// names them rather than being refused, and it is exactly why `drawn` and
/// `occludes` are on the *other* list: routing a rendering field through this
/// byte would buy that cost again for a change no player can act on.
///
/// **Each move costs a player who already paid for the last one.** A world saved
/// after `targetable` joined the list, and quit normally so that its blocks were
/// rewritten under revision 2, was told again under revision 3; a world rewritten
/// under revision 3 is told again here — because each pair of records is folded
/// over different field lists and nothing in either can say the blocks are the
/// same. "Told once" is once *per move*, not once ever, and
/// `docs/user/gameplay.md` is where a player reads that.
///
/// **Only a test that states the byte sequence can see one of these move.**
/// Measured: leaving the appearance byte at 1 while its list grew reddens the two
/// guards that build the expected bytes by hand and nothing else in the
/// workspace. Every other witness compares one fold to another, and that cannot
/// see a leading byte which moved in both — so a green suite is no evidence a
/// revision is right.
/// **And they are equal again, at 4, by routes that still share nothing.** The
/// appearance list has now grown a third time, over how much light a block
/// stops. Two numbers arriving at one value twice, having moved on five
/// separate occasions and never once together, is the clearest evidence
/// available that the equality means nothing — and the second invitation to
/// unify them, which is the same mistake the paragraph above prices.
///
/// **And they part again, at 5 and 4, over the colour a block is seen through
/// from inside.** That is the same test applied a sixth time and answered the
/// same way: what a medium carries a view toward changes nothing about standing
/// on the block, building through it or breaking it, so nobody holding one has
/// anything to decide — and routing it through the behaviour byte would refuse
/// the world of every player who asked to be stopped if anything moved, over a
/// colour that is only ever seen from inside a block they are standing in.
const BEHAVIOUR_REVISION: u8 = 4;
const APPEARANCE_REVISION: u8 = 5;

/// The declared behaviour of a block, as revision 4 of this list defines it.
///
/// **Written out by hand rather than derived from [`BlockDefinition`], and that
/// is the whole of it.** A derive over that type would bind every save to a
/// struct which exists for other reasons and changes for other reasons, so a
/// field added to it in a later engine version would invalidate every world in
/// existence. This list *is* the specification: a new definition field does not
/// reach it, and putting one here bumps [`BEHAVIOUR_REVISION`] and the format
/// version together.
///
/// **The origin is excluded, and it is the field that would have broken
/// everything.** It is a human-readable label derived from the *file path* a
/// definition was read out of, so hashing it would make a save written from a
/// repository at one checkout refuse to load from another — for a reason with
/// nothing to do with content, and with a refusal a player could not tell apart
/// from corruption.
///
/// Defaults are resolved before a definition exists, so what is folded is the
/// resolved value and "declared versus defaulted" is not a distinction the type
/// can make.
///
/// **A new field is appended and never inserted.** The canonical encoding writes
/// a struct positionally, so a rename costs nothing and a field placed among the
/// existing ones moves every byte after it — every save in existence would then
/// disagree for a reason nobody declared, and the revision byte would be
/// reporting a change larger than the one that was made.
///
/// `targetable` is here rather than on the appearance list because it decides
/// what a swing *does* to a world: it is what makes `breakable = false`
/// reachable at all, so a block that becomes aimable is a different block to
/// stand in front of.
///
/// **`swimmable`, `move_resistance` and `swim_ascent` are here for that question
/// asked of a volume rather than of a swing.** Whether a player can hold itself
/// up inside a block, how much that block slows what moves through it, and how
/// fast it carries a swimmer who asks to rise, decide whether walking into it
/// drops you, floats you or barely delays you. None of the three is visible in a
/// still frame, which is the same test that put `drawn` and `occludes` on the
/// other list and puts these three here.
///
/// **`move_resistance` and `swim_ascent` are the only numbers on either list**,
/// and each is folded as the four bytes of its `f32` bit pattern at the width the
/// physics reads it at. A declared `-0.0` is normalised to `0.0` when it is read,
/// so two declarations meaning no resistance — or no lift — cannot fold apart
/// over a sign bit no player wrote.
///
/// **The ascent is folded as it was declared, never masked against
/// `swimmable`.** A non-swimmable block contributes an ascent of `0.0` to the
/// *medium table*, which is what keeps that table one bit wide — but that is a
/// rule about what a volume does to a tick, and this is a record of what a block
/// was declared to be. Two declarations differing only in an ascent are two
/// different blocks to a save whether or not either of them lifts anybody today,
/// because making one of them swimmable later must not silently resurrect a
/// number the record never kept.
#[derive(Serialize)]
struct DeclaredBehaviour<'a> {
    input_version: u8,
    name: &'a str,
    is_solid: bool,
    replaceable: bool,
    breakable: bool,
    breaks_into: Option<&'a str>,
    targetable: bool,
    swimmable: bool,
    move_resistance: f32,
    swim_ascent: f32,
}

/// The declared appearance of a block.
///
/// Separate from the behaviour above, and the split is the point: a block whose
/// texture changed is the same block to stand on, and a block whose solidity or
/// drop changed is not. One value for both would make a retextured mod
/// indistinguishable from a rebalanced one, and the only safe answer to that
/// ambiguity is to report every texture edit — which buries the one report that
/// was worth reading.
///
/// The name is in both lists, so that the two hashes of one block cannot be
/// swapped for each other and a block's appearance cannot collide with some
/// other block's behaviour.
///
/// **Six keys, in [`Face::ALL`] order, and the order is part of the record.** A
/// block whose `north` alone was re-pointed looks different and must fold
/// differently, so the keys are folded where they were declared rather than
/// sorted or reduced to the distinct ones — two blocks that swapped their `east`
/// and `west` art are not the same block seen twice.
///
/// A fixed-size array rather than six named fields: the canonical encoding
/// writes a tuple as its elements and nothing else, so the two shapes fold
/// identically, and this one cannot have a face left out of it.
///
/// **`drawn`, `occludes` and `opacity` are appended after the keys, in that
/// order**, for the positional reason [`DeclaredBehaviour`] records. They are on
/// this list because a block that stopped being drawn, stopped hiding what
/// stands behind it, or started letting light through, is still the same block
/// to stand on, to build through and to break: nothing about mutating the world
/// changes, so nothing a player has to decide about changes either.
///
/// **The degree is folded as the `f32` a declaration stated and never as the
/// byte a vertex carries.** Quantising it here would fold two declarations a
/// code value apart into one record, so an edit a player can see would report
/// the block unchanged — and it would tie the on-disk meaning of a save to an
/// encoding the renderer is free to widen. The `-0.0 → 0.0` normalisation the
/// loader applies is what keeps two spellings of the same degree folding alike.
///
/// **The medium is appended after the degree, as one optional value over the
/// pair.** The loader states the colour and the distance together or not at all,
/// so the record carries that rule in its own shape rather than restating it —
/// and the single tag byte is what separates a block declaring no medium from
/// every colour at every distance, black at no distance included. Two optionals
/// would admit a record shape the loader refuses, and nothing reading one back
/// could tell which of the two absences it was holding.
///
/// **What is folded is what the declaration stated**, which is what makes the
/// two accepted spellings free: `#3A6EA5`, `#3a6ea5` and `#3A6EA5FF` parse to
/// the same three bytes and therefore fold identically, so three spellings of
/// one colour cannot report a block as retextured. The distance folds as the
/// `f32` the loader kept, through the reader that already normalises `-0.0`.
#[derive(Serialize)]
struct DeclaredAppearance<'a> {
    input_version: u8,
    name: &'a str,
    textures: [&'a str; 6],
    drawn: bool,
    occludes: bool,
    opacity: f32,
    tint: Option<DeclaredTint>,
}

/// The medium a declaration stated, as the record folds it.
///
/// The three channel bytes with no length in front of them — a fixed-size array
/// rather than a slice, for the reason [`DeclaredAppearance`]'s keys are one —
/// and the distance's four bytes behind them.
#[derive(Serialize)]
struct DeclaredTint {
    color: [u8; 3],
    distance: f32,
}

/// What revision 4 of the behaviour list records as `definition`'s declared
/// behaviour.
///
/// **Every save written before this revision reports every block it holds as
/// `changed` on its next load**, and that is the designed cost of appending the
/// ascent rather than a migration defect: how fast a block's volume carries a
/// swimmer is part of what that block is, and the two records are not comparable
/// across the move. Such a save loads and names its blocks instead of being
/// refused, which is the only reason the cost is payable at all.
///
/// **Payable is not the same as free, and this is the third consecutive move.**
/// A world saved under revision 2 was already told once, over `targetable`, and
/// again under revision 3 over the two medium fields; it is told a third time
/// here — one report each, and whoever crosses all three pays three times. That
/// is the honest answer rather than a defect: nothing in any pair of records can
/// say the blocks are unchanged, because each pair was folded over a different
/// list.
pub(crate) fn behaviour_of(definition: &BlockDefinition) -> DefinitionHash {
    folded(&DeclaredBehaviour {
        input_version: BEHAVIOUR_REVISION,
        name: definition.name.as_str(),
        is_solid: definition.is_solid,
        replaceable: definition.replaceable,
        breakable: definition.breakable,
        breaks_into: definition.breaks_into.as_ref().map(BlockName::as_str),
        targetable: definition.targetable,
        swimmable: definition.swimmable,
        move_resistance: definition.move_resistance,
        swim_ascent: definition.swim_ascent,
    })
}

/// What revision 5 of the appearance list records as `definition`'s declared
/// appearance.
///
/// **Every save written before this revision reports every block as retextured
/// on its next load, and that is correct rather than a migration defect**: every
/// block's appearance really did change. A retexture is loaded with nothing said
/// about it, which is exactly why the two lists carry separate revision bytes —
/// a byte shared between them would have turned an added texture key, or a
/// drawnness nobody can act on, into a claim that every block in existence
/// behaves differently.
pub(crate) fn appearance_of(definition: &BlockDefinition) -> DefinitionHash {
    folded(&DeclaredAppearance {
        input_version: APPEARANCE_REVISION,
        name: definition.name.as_str(),
        textures: Face::ALL.map(|face| definition.textures.at(face).as_str()),
        drawn: definition.drawn,
        occludes: definition.occludes,
        opacity: definition.opacity.get(),
        tint: definition.tint.map(|declared| DeclaredTint {
            color: declared.color(),
            distance: declared.distance(),
        }),
    })
}

/// `declaration` in its canonical bytes, folded into 64 bits.
///
/// The encoding is the file's own, which is what gives every variable-length
/// field its length prefix — so `("ab", "c")` and `("a", "bc")` cannot fold
/// identically.
///
/// **Total, deliberately.** Encoding one of the two owned declarations below into
/// a growable vector has no reachable failure: their shape is fixed, every field
/// is a type the encoder always accepts, and the destination cannot fill up. The
/// fallback is written as a fallback and not an unwrap because a panic here would
/// end a save — a total function is the point, not the particular way the encoder
/// could once decline.
fn folded(declaration: &impl Serialize) -> DefinitionHash {
    let canonical = postcard::to_stdvec(declaration).unwrap_or_default();
    DefinitionHash::from_raw(fnv_1a_64(&canonical))
}
