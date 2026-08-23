//! Whether a candidate registry may replace the one a world is named against,
//! and the swap when it may.
//!
//! **A child of [`world`](super) and not a sibling, and that is load-bearing.**
//! `World::adopt` carries no `pub`, so it is visible here and nowhere else — the
//! arrangement [`action`](super::action) already has. `pub(crate)` instead would
//! open a crate-wide second write door and every test would stay green.

use std::collections::BTreeMap;
use std::sync::Arc;

use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::content::FaceTextures;
use mc_core::id::BlockName;

use crate::reload::ReloadRefusal;
use crate::world::World;
use crate::world::action::default_held_block;

/// What admitting a candidate settled, beyond the world it settled it in.
pub(crate) struct Adopted {
    /// The block a client holds under the content now serving, re-derived.
    pub holding: BlockName,
}

/// Whether `registry` may replace the one `world` is named against, and the swap
/// when it may.
///
/// Both refusals are decided against the world as it is now — a player can place
/// a block while a candidate is being built — and both are decided before
/// anything is written.
///
/// # Errors
///
/// Returns [`ReloadRefusal::BlocksTheWorldHolds`] naming every block some cell
/// still holds that `registry` does not declare, and
/// [`ReloadRefusal::NothingToPlace`] where it declares no solid block at all —
/// in both cases with `world` untouched.
pub(crate) fn adopt_candidate(
    world: &mut World,
    registry: Arc<BlockRegistry>,
) -> Result<Adopted, ReloadRefusal> {
    let undeclared: Vec<BlockName> = world
        .names_held()
        .into_iter()
        .filter(|held| registry.resolve(held).is_err())
        .cloned()
        .collect();
    if !undeclared.is_empty() {
        return Err(ReloadRefusal::BlocksTheWorldHolds { blocks: undeclared });
    }
    let holding = default_held_block(&registry).ok_or(ReloadRefusal::NothingToPlace)?;
    let redraws = changes_geometry(world.registry(), &registry);
    world
        .adopt(registry)
        .map_err(|refused| refused_over(&refused))?;
    if redraws {
        world.mark_every_section();
    }
    Ok(Adopted { holding })
}

/// Whether replacing `serving` with `candidate` changes what is drawn.
///
/// Binary, and the marking it drives is all-or-nothing: whether a block is
/// drawn, whether it hides what stands behind it, the keys the six faces draw
/// from, or the set of names. `solid`, `targetable`, `replaceable`, `breakable`
/// and `breaks_into` change no geometry. Narrowing this to the sections that hold
/// a changed name marks about 82 of 256 in the shipped world and fails the spec's
/// stated bound, so it is a spec change rather than an optimisation.
fn changes_geometry(serving: &BlockRegistry, candidate: &BlockRegistry) -> bool {
    drawn_of(serving) != drawn_of(candidate)
}

/// Every block's name against the three fields that decide what is drawn.
///
/// A map rather than a list, so a re-ordered declaration is not mistaken for a
/// geometry change.
///
/// **Solidity is deliberately not among them, and its removal looks like a
/// regression until you read what replaced it.** This keyed on `is_solid` while
/// one bit answered every question about a block, so it stood in for drawnness
/// rather than meaning collision. Now that a declaration states the two apart,
/// keeping solidity here would rebuild all 256 sections for a physics edit that
/// changes not one pixel. A declaration stating `solid` and nothing else still
/// marks the world, because `drawn` and `occludes` default to it — so the shipped
/// fixtures are unaffected, and the reason is the loader's default rather than
/// this key.
///
/// **`targetable` is not among them either.** What a swing can find is not
/// something a section could show.
///
/// **All six keys and not one.** A block whose `north` alone was re-pointed is a
/// block that draws differently, and comparing a single key would leave such a
/// reload marking nothing at all — the edit would be accepted and no section
/// would be built again to show it.
fn drawn_of(registry: &BlockRegistry) -> BTreeMap<&BlockName, (bool, bool, &FaceTextures)> {
    (0..registry.registered_count())
        .filter_map(|position| u32::try_from(position).ok())
        .filter_map(|raw| registry.definition(BlockId::from_raw(raw)).ok())
        .map(|declared| {
            (
                &declared.name,
                (declared.drawn, declared.occludes, &declared.textures),
            )
        })
        .collect()
}

/// The refusal a resolve stopped at, said in the admission's own vocabulary.
///
/// Unreachable — the names-held check above already established it. Reported
/// rather than unwrapped because a panic on the tick thread is the one outcome a
/// reload must not have.
fn refused_over(refused: &RegistryError) -> ReloadRefusal {
    ReloadRefusal::BlocksTheWorldHolds {
        blocks: match refused {
            RegistryError::UnknownName { name } => vec![name.clone()],
            _ => Vec::new(),
        },
    }
}
