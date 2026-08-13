//! Guard. The file a save is completed in is a sibling of the save.
//!
//! Every atomicity guarantee this module makes rests on one platform fact: a
//! rename replaces an existing target in a single step **within one volume**,
//! and across volumes becomes a copy and a delete — which is precisely the
//! window the whole arrangement exists to close. Same directory means same
//! volume, so "beside the target" is not tidiness, it is the precondition.
//!
//! **Nothing else holds it.** The scenario that reads like the guard here is the
//! one counting what the save's directory holds once a save has finished, and it
//! was measured not to be: with the sibling moved to the system temporary
//! directory it stayed green, because a leftover landing somewhere else
//! satisfies "exactly one entry in this directory" exactly as trivially as an
//! in-place overwrite does. What that scenario really verifies is that the
//! rename *replaces*. This is the missing half, and it is a unit test because
//! the path is chosen by a private function and never surfaces anywhere a caller
//! can see it.
//!
//! The two tests share one predicate, and the second is a **positive control**
//! rather than a spare assertion. A test that only says "these two directories
//! are the same" goes green forever the day the comparison stops being able to
//! say anything — so the same predicate is asked about a path that is genuinely
//! somewhere else, and has to say so.
//!
//! Both are lexical: no file is created, nothing is renamed, and no temporary
//! directory is involved. The claim is about which directory a path names, and
//! putting a filesystem in the way of it would only add a way for it to be flaky.

use std::path::{Path, PathBuf};

use super::sibling_of;

/// The save these tests are about.
///
/// Relative and never touched, because nothing here reaches a disk.
fn a_save() -> PathBuf {
    Path::new("saves").join("world.mcw")
}

/// A staging file somewhere other than beside the save.
///
/// The shape the measured mutation took: a temporary file put wherever temporary
/// files usually go, which is a different directory and, on a machine with more
/// than one drive, routinely a different volume.
fn somewhere_else() -> PathBuf {
    Path::new("elsewhere").join("world.mcw.tmp")
}

/// Whether `staged` sits in the same directory as `save`.
///
/// The whole of the property, written once so that the control below asks the
/// same question the assertion above does — a control exercising a second,
/// similar predicate would prove that predicate works and nothing about this one.
fn beside(save: &Path, staged: &Path) -> bool {
    staged.parent() == save.parent()
}

#[test]
fn the_file_a_save_is_completed_in_is_written_beside_the_save() {
    let save = a_save();

    let staged = sibling_of(&save).ok();

    assert_eq!(
        staged.as_deref().map(|staged| beside(&save, staged)),
        Some(true),
        "a rename is atomic only within one volume, so the file a save is completed in has to \
         share the save's directory — and a staging file in the system temporary directory is on \
         another volume the moment a player keeps their worlds on a second drive. The rename then \
         degrades into a copy and a delete, and a machine stopping in that window costs them both \
         the old world and the new one, which is the single failure this whole path was built to \
         make impossible"
    );
}

#[test]
fn a_staging_file_in_another_directory_is_reported_as_not_beside_the_save() {
    let save = a_save();

    assert!(
        !beside(&save, &somewhere_else()),
        "the control. The test above asserts an absence of difference, and an absence goes green \
         forever the day the check stops being able to find one — so the same predicate is handed \
         a path that really is somewhere else and has to say so. Without this, a comparison that \
         had quietly become vacuous would report the guarantee holding on the day it stopped"
    );
}
