//! What applying a definition source does when the source itself fails part-way
//! through yielding its definitions.
//!
//! A source is a stream, so a failure can arrive after some definitions have
//! already been handed over. Applying one is all-or-nothing: a mod author whose
//! second file is broken must not end up with half a mod loaded.

mod common;

use std::error::Error;

use common::{TestResult, definition};
use mc_core::block::source::{DefinitionFault, DefinitionSourceError, InMemoryDefinitionSource};
use mc_core::block::{BlockRegistry, DefinitionOrigin, RegistryError};

/// What the source reports in place of its second definition.
const MID_STREAM_CAUSE: &str = "the definition could not be read";

/// A source that yields one definition, then reports a failure, then would
/// yield one more. The definition *before* the failure is what makes a
/// register-as-you-go implementation observable.
fn source_failing_at_its_second_definition() -> Result<InMemoryDefinitionSource, Box<dyn Error>> {
    let fault = DefinitionFault {
        origin: DefinitionOrigin::new("second.toml"),
        block: Some("fixture:second".to_owned()),
        field: None,
        cause: MID_STREAM_CAUSE.to_owned(),
    };
    Ok(InMemoryDefinitionSource::new(
        DefinitionOrigin::new("fixture-content"),
        vec![
            Ok(definition("fixture:first", "fixture:first", "first.toml")?),
            Err(DefinitionSourceError::Malformed(fault)),
            Ok(definition("fixture:third", "fixture:third", "third.toml")?),
        ],
    ))
}

#[test]
fn a_source_that_fails_mid_stream_registers_none_of_its_definitions() -> TestResult {
    let mut registry = BlockRegistry::new();

    let error = registry
        .apply(&source_failing_at_its_second_definition()?)
        .err()
        .ok_or("a source that reports a failure must not apply cleanly")?;

    let RegistryError::Source(DefinitionSourceError::Malformed(reported)) = &error else {
        return Err(
            format!("expected the source's own failure to propagate, got {error:?}").into(),
        );
    };
    assert_eq!(
        reported.cause.as_str(),
        MID_STREAM_CAUSE,
        "the failure the source reported is the one the caller is handed"
    );
    assert_eq!(
        registry.registered_count(),
        0,
        "the definition yielded before the failure must not have been registered"
    );
    Ok(())
}
