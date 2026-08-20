//! A set the client cannot read at all, and what it tells whoever built it.
//!
//! # A different axis from what the set *is*
//!
//! `SetVerdict` answers "what is the state of this set" and every one of its
//! arms is an answer. These two readings are about a set that admits no answer:
//! the index is there, it was written by a real build, and reading it is
//! refused. Neither is a verdict, which is why both arrive as an error and why
//! they live in a file of their own.
//!
//! # Both of these are reachable from a manifest somebody can write today
//!
//! The first is a deferred observation the build carries: a manifest naming a
//! model outside its own directory builds cleanly and writes a `source` record
//! the reader then refuses. It was left unbuilt on the stated grounds that the
//! client gives a clear refusal naming the path — so this is that promise, held
//! to rather than assumed.
//!
//! The second is the other half of the rule that says an image file name is
//! derived once and checked on both sides. The build refuses a name it would
//! derive badly; the reader takes the name **from the index** rather than
//! deriving it a second time, and refuses one that fails the same rule. Without
//! that, a name out of a file is joined onto a path — and a set built by an
//! older or a patched tool is a set whose index the client believes.
//!
//! # The two indexes are shaped in `support/built_sets.rs`
//!
//! Both go through `TextureSetIndex::stating` and `rendered`, which is the
//! writer a build uses, and both are built by the fixture module rather than
//! here — because the guard that holds the modding page to these refusals needs
//! exactly the same two roots, and a second copy of either would be a second
//! opinion about what a build emits.

use mc_client::textures::{TextureSetError, built_set};

mod support;

use support::{TestResult, built_sets, refusal_printed_over};

#[test]
fn an_index_recording_a_source_outside_the_content_root_is_refused_naming_the_path() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::recording_a_source_outside_the_root(root.path())?;

    let read = built_set(root.path());

    assert!(
        matches!(read, Err(TextureSetError::Index { .. })),
        "an index recording a source that leaves the content root cannot be read, and the client \
         answered {read:?}"
    );
    let printed = refusal_printed_over(root.path())?;
    assert!(
        printed.contains(built_sets::A_SOURCE_OUTSIDE_THE_ROOT),
        "whoever wrote a manifest naming a model outside its own directory has one thing to \
         change, and the refusal has to name it. It said: {printed}"
    );
    Ok(())
}

#[test]
fn an_index_naming_an_image_that_is_not_an_ordinary_name_is_refused_naming_it() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::naming_one_image(root.path(), built_sets::AN_IMAGE_NAME_THAT_IS_A_PATH)?;

    let read = built_set(root.path());

    assert!(
        matches!(read, Err(TextureSetError::UnusableImageName { .. })),
        "a name taken out of an index is joined onto a path, so it is checked against the rule \
         the build derived it under. The client answered {read:?}"
    );
    let printed = refusal_printed_over(root.path())?;
    assert!(
        printed.contains(built_sets::AN_IMAGE_NAME_THAT_IS_A_PATH),
        "the refusal has to name the record at fault, and it said: {printed}"
    );
    Ok(())
}
