//! The byte fold that two independent programs have to agree on forever.
//!
//! # Why it sits in this crate and not beside its first caller
//!
//! The fold's first consumer is the save format in `mc-world`, which records a
//! 64-bit hash per block declaration. Its second is `voxforge`, the art build
//! under `tools/`, whose texture-set index has to fold to the same value from
//! the same bytes as the client that reads it — otherwise a build and a client
//! disagree about whether a set is current, and the disagreement is silent.
//!
//! `tools/` may depend inward on `crates/`; the reverse never holds, and that
//! is mechanically asserted. So the only place both sides can reach one
//! implementation is the crate everything already depends on. **A second
//! implementation on either side of that line is the defect this arrangement
//! exists to make unspellable** — not a hash that is wrong, but two hashes that
//! were each computed correctly and do not match.
//!
//! The index contract, and the byte sequence it folds, live in [`crate::art`] —
//! the magic line, the `fold <16 hex digits>` record, the source lines it folds
//! over, and `TextureSetIndex::parse` reading them back.

/// Where an FNV-1a 64 fold starts, and what it multiplies by.
///
/// Published constants, fixed for good: a hash that moved would report every
/// block of every existing save as changed.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// `bytes` folded with FNV-1a 64.
///
/// Hand-written, and deliberately not the standard library's default hasher:
/// that algorithm is documented as unspecified and may change between compiler
/// releases, and a hash that moves with the toolchain invalidates every save on
/// an upgrade. It is also what lets two programs built from different trees
/// share one value. Not a cryptographic hash either — forgery resistance buys
/// nothing for a local file a player can already edit, and it would make the
/// expected value of a hash impossible to derive by hand, which is the one thing
/// the version-stability test cannot do without.
///
/// Nothing here parses. There is no length to trust, no allocation to drive and
/// no index to bound — every byte handed in is read once and folded — which is
/// why hand-writing *this* is not the thing hand-writing a decoder would have
/// been.
pub fn fnv_1a_64(bytes: &[u8]) -> u64 {
    let mut folded = FNV_OFFSET_BASIS;
    for byte in bytes {
        folded ^= u64::from(*byte);
        folded = folded.wrapping_mul(FNV_PRIME);
    }
    folded
}
