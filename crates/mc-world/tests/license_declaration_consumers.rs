//! The two places the licence declaration has to reach.
//!
//! `license_declaration.rs` grades what an expression requires.
//! `shipped_license_texts.rs` grades whether the files in the tree are the
//! licences. Neither of them asks the two questions a reader actually runs
//! into: does every crate in this workspace offer the same terms, and does the
//! README point at texts that are there.
//!
//! # The member set is a derived oracle, never a count
//!
//! `crates/` holds one directory per member, so that is what the metadata read
//! is compared against — not the literal `10` today's tree would produce. A
//! committed count goes red against a *correct* property the day an eleventh
//! crate lands, and a reviewer's cheapest green is editing the number. The
//! directories are read here, in the test, by code that shares nothing with the
//! metadata read it grades.
//!
//! The oracle refuses to be empty, in [`crate_directories`]. Both readings
//! below would otherwise agree with a metadata read that resolved nothing, and
//! agree with it silently — which is the failure this whole check exists to
//! catch in someone else's code.
//!
//! # A member declaring nothing is the vacuity hole
//!
//! "Every member declares the same expression" is satisfied by a member that
//! declares none, if a check counts an absent declaration as agreement.
//! [`a_member_declaring_no_licence_at_all_is_reported_as_undeclared_rather_than_matching`]
//! is the control that closes it, and it expects the refusal
//! `super::license`'s own reading produces — an implementation that decided
//! "declares nothing" a second time here would have to agree with the first
//! one by hand.
//!
//! # What the README fixtures are built to catch
//!
//! Three properties of a licence-link detector are spec-defined, and two of
//! them are invisible to a fixture that ignores them. So the fixture for a dead
//! link carries **one link in the angle-bracket `[text](<TARGET>)` form**, and
//! carries it **outside the `## License` section**. A detector reading only the
//! plain form, or only that section's body, collects one of the two links
//! instead of both — and then reports a clean all-clear for a README whose
//! licence link is dead, which is the exact defect the scenario is about.
//!
//! # The expected filenames are written here, not imported
//!
//! `LICENSE-MIT` and `LICENSE-APACHE` are literals below rather than values
//! read back from the code under test. An expectation that takes its answer
//! from the thing that produced it agrees with any answer at all.

mod common;

use std::error::Error;
use std::fs;

use common::license::{Declared, Refusal};
use common::license_consumers::{
    DivergingMember, Links, Members, license_links, member_licenses, resolved_workspace_metadata,
    workspace_members,
};
use common::{TestResult, repository_root};
use serde_json::Value;
use tempfile::TempDir;

/// The expression this workspace declares, and the one every member inherits.
const DUAL_LICENSE: &str = "MIT OR Apache-2.0";

/// The directory holding one subdirectory per workspace member.
const CRATES: &str = "crates";

/// The file the MIT licence ships as.
const MIT_FILE: &str = "LICENSE-MIT";

/// The file the Apache License 2.0 ships as.
const APACHE_FILE: &str = "LICENSE-APACHE";

/// The file a reader arrives at first.
const README_FILE: &str = "README.md";

/// The member whose declaration the two controls alter.
///
/// `mc-core` depends on nothing in the workspace and everything depends on it,
/// so it is the member least likely to be renamed out from under a fixture. The
/// controls change what it declares and nothing else, so the altered document
/// and the real one differ in one field.
const ALTERED_MEMBER: &str = "mc-core";

/// An expression no crate in this workspace declares, and one that could not be
/// mistaken for a typo of the one they all do.
const FOREIGN_EXPRESSION: &str = "GPL-3.0-only";

#[test]
fn the_workspace_members_cargo_resolves_are_the_crate_directories() -> TestResult {
    let expected = crate_directories()?;

    assert_eq!(
        workspace_members(&resolved_workspace_metadata()?),
        expected,
        "the member set is compared against the directories under {CRATES}/ rather than against a \
         number, so it guards a metadata read that resolved nothing and still holds the day an \
         eleventh crate lands. The resolved document also carries every third-party package cargo \
         resolved, hundreds of which declare this same expression — so a read that took `packages` \
         for the member list would look like agreement while grading crates this workspace does \
         not own"
    );
    Ok(())
}

#[test]
fn every_workspace_member_resolves_the_licence_expression_the_workspace_declares() -> TestResult {
    let members = crate_directories()?;

    assert_eq!(
        member_licenses(&resolved_workspace_metadata()?, DUAL_LICENSE),
        Members::AllDeclaring(members),
        "a dual licence offered by nine crates out of ten is not the offer the manifest makes, and \
         the members are inheriting one workspace value rather than repeating it — so this reads \
         the resolved value per member. The verdict names the members it graded: 'they all agree' \
         is trivially true of no members at all, which is precisely what a read that resolved \
         nothing produces"
    );
    Ok(())
}

#[test]
fn a_member_declaring_another_expression_is_reported_with_what_it_declared() -> TestResult {
    let altered = with_member_license(
        &resolved_workspace_metadata()?,
        ALTERED_MEMBER,
        Some(FOREIGN_EXPRESSION),
    );

    assert_eq!(
        member_licenses(&altered, DUAL_LICENSE),
        Members::Diverging(vec![DivergingMember {
            member: ALTERED_MEMBER.to_owned(),
            declared: Declared::Expression(FOREIGN_EXPRESSION.to_owned()),
        }]),
        "the reading above is an agreement, and an agreement proves nothing unless the same check \
         can be shown finding a disagreement. It is the real resolved document with one field \
         changed, so the control differs from the subject in the declaration and in nothing else. \
         Naming the member without the expression it declared would leave a reader unable to tell \
         a stale crate from a deliberately relicensed one, and the nine that still agree are not \
         the finding"
    );
    Ok(())
}

#[test]
fn a_member_declaring_no_licence_at_all_is_reported_as_undeclared_rather_than_matching()
-> TestResult {
    let altered = with_member_license(&resolved_workspace_metadata()?, ALTERED_MEMBER, None);

    assert_eq!(
        member_licenses(&altered, DUAL_LICENSE),
        Members::Diverging(vec![DivergingMember {
            member: ALTERED_MEMBER.to_owned(),
            declared: Declared::Refused(Refusal::NoDeclaration(ALTERED_MEMBER.to_owned())),
        }]),
        "this is the vacuity hole in 'every member declares the same thing': a member that \
         declares nothing contradicts nothing, so a check comparing only the members that did \
         declare something reports agreement across a workspace one of whose crates offers no \
         terms at all. The expected refusal is the one the declaration reading already produces, \
         so an implementation deciding here for a second time what 'declares nothing' means would \
         have to reproduce that answer by hand"
    );
    Ok(())
}

#[test]
fn the_shipped_readme_links_both_licence_texts_and_both_resolve() -> TestResult {
    let root = repository_root()?;
    let readme = fs::read_to_string(root.join(README_FILE))?;

    assert_eq!(
        license_links(&readme, &root)?,
        Links::AllResolve(vec![APACHE_FILE.to_owned(), MIT_FILE.to_owned()]),
        "the README is where a reader looks before a manifest, and prose saying the texts are not \
         in the tree is exactly what has to stop being true. Both targets are named rather than \
         counted, so a scan that collected one link cannot report the same all-clear as one that \
         collected both — and they are named in file order rather than in the order the prose \
         happens to introduce them, so this asserts what the README links and not how it is worded"
    );
    Ok(())
}

#[test]
fn a_readme_link_to_a_licence_file_absent_from_the_tree_is_reported_by_target() -> TestResult {
    let root = a_root_holding(&[APACHE_FILE])?;
    let readme = format!(
        "# A fixture\n\nContributions fall under [the MIT terms](<{MIT_FILE}>) unless stated \
         otherwise.\n\n## License\n\nDual-licensed, and [the Apache terms]({APACHE_FILE}) are \
         here.\n"
    );

    assert_eq!(
        license_links(&readme, root.path())?,
        Links::Unresolved(vec![MIT_FILE.to_owned()]),
        "the reading above is an absence of dead links, and an absence proves nothing unless the \
         same scan can be shown finding one — the root holds the Apache text and not the MIT one, \
         so a scan that reported both or neither would be reporting something other than what it \
         looked for. The dead link is deliberately the harder of the two to collect: it is written \
         in the angle-bracket form, and it sits outside the licence section, both of which the \
         spec defines as counting. A detector reading only plain links, or only that section's \
         body, misses it and hands back a clean verdict for a README whose licence link is dead"
    );
    Ok(())
}

#[test]
fn a_readme_with_no_licence_link_refuses_rather_than_reporting_every_link_resolved() -> TestResult {
    let root = a_root_holding(&[MIT_FILE, APACHE_FILE])?;

    assert_eq!(
        license_links(README_WITHOUT_A_LICENCE_LINK, root.path())?,
        Links::NoneLinked,
        "'every licence link resolves' is trivially true of a README that links no licence, so a \
         check answering the all-clear there goes green forever the day someone rewrites the \
         section. The root holds both texts, which is what keeps this from being read as a \
         complaint about a bare tree, and the README links three things that are not licences — \
         so a detector that merely counted links, or one that collected nothing at all, would \
         answer this the same way and be wrong for opposite reasons"
    );
    Ok(())
}

/// A README that links things, none of them a licence file.
///
/// The three targets are what an ordinary README links, spread across the two
/// link forms: nothing here has a final path segment beginning with `LICENSE`,
/// and a detector that answered anything but a refusal for it would be reading
/// links rather than licence links.
const README_WITHOUT_A_LICENCE_LINK: &str = "\
# A fixture

Built with [the framework](https://github.com/prodigy-sln/prospect).

## License

Dual-licensed under MIT or Apache-2.0, as declared in `Cargo.toml`.

See [the registry](specs/REGISTRY.md) and [the standards](<standards/global>).
";

/// The name of every directory under `crates/`, in name order — one per
/// workspace member.
///
/// The oracle the resolved member set is graded against, and it shares no code
/// with the metadata read: a directory listing cannot agree with a broken
/// metadata read by making the same mistake.
///
/// # Errors
///
/// Returns the I/O failure when the directory cannot be read, and an error when
/// it holds no crate at all — an empty oracle would agree with a metadata read
/// that resolved nothing, which is the one answer these tests exist to reject.
fn crate_directories() -> Result<Vec<String>, Box<dyn Error>> {
    let mut named = Vec::new();
    for entry in fs::read_dir(repository_root()?.join(CRATES))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            named.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    named.sort();
    if named.is_empty() {
        return Err(
            format!("{CRATES}/ holds no crate, so the oracle below would be vacuous").into(),
        );
    }
    Ok(named)
}

/// `metadata` with `member`'s licence declaration replaced by `expression`, or
/// removed where none is given.
///
/// A transformation of the real resolved document rather than a hand-built one,
/// so the control and the subject differ in the declaration and in nothing
/// else. A run in which it altered no package leaves the document saying what
/// it already said, and the assertion that reads it fails — there is no
/// spelling of that mistake in which this quietly grades the wrong thing.
fn with_member_license(metadata: &Value, member: &str, expression: Option<&str>) -> Value {
    let mut altered = metadata.clone();
    for package in packages_of(&mut altered) {
        if package.get("name").and_then(Value::as_str) == Some(member) {
            set_license(package, expression);
        }
    }
    altered
}

/// Every package `metadata` describes, ready to be altered.
fn packages_of(metadata: &mut Value) -> impl Iterator<Item = &mut Value> {
    metadata
        .get_mut("packages")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
}

/// Declares `expression` for `package`, or removes its declaration outright
/// where none is given.
///
/// Removing the key is the sharper of the two spellings cargo uses for "this
/// package declares no licence": a value of `null` at least looks like a field
/// that was consulted.
fn set_license(package: &mut Value, expression: Option<&str>) {
    let Some(fields) = package.as_object_mut() else {
        return;
    };
    match expression {
        Some(expression) => {
            fields.insert("license".to_owned(), Value::String(expression.to_owned()))
        }
        None => fields.remove("license"),
    };
}

/// A temporary root holding each of `files`.
///
/// The bodies are placeholder prose and deliberately not empty: whether a text
/// *is* the licence is graded by `shipped_license_texts.rs` against the shipped
/// file, and a link check that resolved a zero-length file would be
/// indistinguishable here from one that resolved a real text.
///
/// # Errors
///
/// Returns the I/O failure when the temporary root cannot be written.
fn a_root_holding(files: &[&str]) -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    for file in files {
        fs::write(directory.path().join(file), "the text of a licence\n")?;
    }
    Ok(directory)
}
