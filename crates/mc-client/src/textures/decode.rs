//! Turning a built set's images into the texels an array-texture layer is
//! filled from.
//!
//! # This is the only file in this crate that may name the image decoder
//!
//! Swapping the decoder is then one edit, and every other file of the
//! composition root stays a file about content rather than about a format.
//! `tests/the_decode_stays_at_the_composition_root.rs` states the path rather
//! than a count, so a decoder that moved is a different answer instead of the
//! same one.
//!
//! **And it is the client that decodes, not the renderer.** `mc-render` has no
//! filesystem type anywhere in its source and this spec does not give it one: it
//! gains the ability to be *handed* level-zero texels per key. Pixels are the
//! client's half of the split, and a server that needed them would break the
//! asymmetry that makes a texture pack a legal client modification and a block
//! declaration not.
//!
//! # Both refusals name the key, and that is the whole point of them
//!
//! Whoever meets one of these has a texture *key* in a manifest and a *file* in
//! a directory, and only the key connects the file back to the declaration that
//! wanted it. A message about `base__stone.png` with no key in it hands a mod
//! author a filename they never typed.
//!
//! # Why a derived, gitignored directory is checked at all
//!
//! The build already refuses a model whose scale and pixels-per-voxel do not
//! come to a block texture's edge, naming the model — which is what its author
//! can fix. None of that is a reason to trust the directory: it is an ordinary
//! directory on somebody's disk, and a set built by an older tool, a patched one
//! or a hand-edited one is a set this client is handed all the same. Uploading a
//! 32 x 32 image into a 16 x 16 layer is a buffer overrun.

use std::path::Path;

use image::ImageFormat;
use mc_core::art::TextureSetIndex;
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::texture::supplied::SuppliedTexels;

use super::{TextureSetError, index};

/// The texels every image `recorded` names offers, one entry per key.
///
/// **Every entry, including a key no block in this root declares.** A set built
/// for content that has since dropped a block is the ordinary way there, and the
/// image simply occupies no layer; refusing it would make every set stale the
/// moment a declaration was deleted.
///
/// # Errors
///
/// Returns [`TextureSetError::Unreadable`] where an image the index names cannot
/// be opened, [`TextureSetError::NotAPng`] where its bytes are not a PNG, and
/// [`TextureSetError::Size`] where it is not the edge a layer holds.
pub fn texels_of(
    root: &Path,
    recorded: &TextureSetIndex,
) -> Result<SuppliedTexels, TextureSetError> {
    let mut supplied = Vec::with_capacity(recorded.entries().len());
    for entry in recorded.entries() {
        let at = index::image_path(root, &entry.image);
        let bytes = index::image_bytes(root, &entry.image)?;
        supplied.push((entry.key.clone(), texels_in(&entry.key, &at, &bytes)?));
    }
    Ok(SuppliedTexels::stating(supplied))
}

/// The level-zero texels `bytes` holds for `key`, in `[R, G, B, A]` stored
/// bytes, row-major.
///
/// The format is stated rather than guessed. A set holds PNGs because that is
/// what the build writes, and guessing would turn "this file is not an image"
/// into "this file is some other image", which is a different message and a
/// worse one.
fn texels_in(key: &TextureKey, at: &Path, bytes: &[u8]) -> Result<Vec<[u8; 4]>, TextureSetError> {
    let decoded = image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .map_err(|_cause| TextureSetError::NotAPng {
            key: key.clone(),
            image: at.to_path_buf(),
        })?
        .to_rgba8();
    let found = decoded.dimensions();
    if found != (TEXTURE_EDGE, TEXTURE_EDGE) {
        return Err(TextureSetError::Size {
            key: key.clone(),
            found,
        });
    }
    Ok(decoded
        .pixels()
        .map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            [red, green, blue, alpha]
        })
        .collect())
}
