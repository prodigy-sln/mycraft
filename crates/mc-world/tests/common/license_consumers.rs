//! Where the declaration has to reach: every workspace member, and the README.
//!
//! Its neighbour [`super::license`] answers what an expression requires. This
//! module answers whether the two places that have to agree with it actually
//! do — the resolved metadata of every crate in the workspace, and the file a
//! reader arrives at first.
//!
//! # A separate module from the derivation it consumes
//!
//! Not because the concerns are unrelated — because the gate caps a file at 600
//! lines and `license.rs` is the derivation plus its refusal contract. The
//! consumers are read by one test binary where the derivation is read by two,
//! so the split costs nothing and leaves a finished, green file alone.
//! [`super::license::declared_expression`] is reused rather than reimplemented:
//! a second reading of "declares nothing" is a second opinion about it, and
//! this module has no business holding one.
//!
//! # `cargo metadata` is invoked here and nowhere else
//!
//! [`resolved_workspace_metadata`] is the single invocation in the whole
//! feature. Everything else takes a value: the two controls below are the same
//! resolved document with one member's declaration altered, which is what makes
//! them controls over this code path rather than fixtures grading a
//! reimplementation of it. `--locked` cannot resolve a fixture workspace in a
//! `TempDir` — there is no `Cargo.lock` — so a check that only accepted a
//! directory would have left both controls unwritable.
//!
//! # Every verdict names the set it graded
//!
//! "They all agree" and "every link resolves" are both trivially true of
//! nothing at all, and nothing at all is exactly what a metadata read that
//! resolved none and a link scan that collected none produce. So the good
//! verdicts carry their members and their targets, and the README check has a
//! third variant for the case where it found no link to grade.
//!
//! # This module is the implementation's, not the test author's
//!
//! It was committed at RED as a deliberately empty skeleton so every driver
//! failed on an assertion rather than on a missing symbol, and
//! [`license_links`] was empty in the *over-eager* direction — the vacuous
//! all-clear over no links at all — because an under-eager skeleton would have
//! satisfied the zero-set guard for precisely the reason that guard exists to
//! forbid.
//!
//! # Two properties here are knowingly ungraded
//!
//! Both were measured by breaking them and watching nothing go red, and both
//! are left standing rather than quietly removed. The name sort in
//! [`workspace_members`] holds incidentally — `cargo metadata` already emits
//! `packages` in name order — so no verdict depends on it. And matching a link
//! target's whole string against `LICENSE` instead of its final path segment
//! changes no verdict either, because every target in the shipped README and in
//! both fixtures is a bare filename; a target like `docs/LICENSE-MIT` is
//! therefore ungraded. The spec defines both properties, so both are
//! implemented as it defines them; neither may be relied on as tested.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::license::{Declared, declared_expression, package_name};

/// What the members of a workspace declare, measured against the expression the
/// workspace itself does.
#[derive(Debug, PartialEq, Eq)]
pub enum Members {
    /// Every member named resolves the expression the check was given, in name
    /// order.
    ///
    /// It carries the members rather than reporting a bare all-clear: a
    /// metadata read that resolved nothing agrees with any expression at all,
    /// and does it silently.
    AllDeclaring(Vec<String>),
    /// The members resolving something else — or nothing — in name order.
    Diverging(Vec<DivergingMember>),
}

/// A member whose declaration is not the workspace's.
#[derive(Debug, PartialEq, Eq)]
pub struct DivergingMember {
    /// The member, by the name its package declares.
    pub member: String,
    /// What its resolved metadata declares: another expression, or a refusal
    /// naming it as declaring none.
    ///
    /// Read through [`super::license::declared_expression`] rather than decided
    /// here, so an absent `license` key and a `"license": null` stay one fact
    /// across the whole feature.
    pub declared: Declared,
}

/// What the licence-file links in a README amount to.
#[derive(Debug, PartialEq, Eq)]
pub enum Links {
    /// Every licence file the README links is in the tree — the targets, named
    /// once each in name order.
    AllResolve(Vec<String>),
    /// The licence-file links naming something the root does not hold, as the
    /// README wrote the target, in name order.
    Unresolved(Vec<String>),
    /// The README carries no licence-file link at all, so there was nothing to
    /// resolve and no all-clear to give.
    NoneLinked,
}

/// Cargo's resolved metadata for this workspace.
///
/// **The one place in this feature that invokes `cargo metadata`.** Invoke it
/// through the `CARGO` variable cargo sets for test binaries, the way
/// `dependency_graph.rs` does, so the same toolchain that built the test
/// resolves the metadata; the invocation is `metadata --format-version 1
/// --locked`.
///
/// # Errors
///
/// Returns an error when cargo cannot be run, when it fails, or when its output
/// is not the metadata document.
pub fn resolved_workspace_metadata() -> Result<Value, Box<dyn Error>> {
    let cargo = std::env::var("CARGO")?;
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {stderr}").into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// The workspace members `metadata` resolves, by package name, in name order.
///
/// The members are the packages `workspace_members` names, **not** every
/// package in the document: `packages` carries the entire resolved dependency
/// graph, and a large share of those third-party crates declare this
/// workspace's own expression — so a check reading `packages` wholesale
/// produces a member set of hundreds that still looks like agreement.
///
/// Name order rather than manifest order, so a caller's expectation does not
/// depend on how `[workspace] members` happens to be sorted today.
#[must_use]
pub fn workspace_members(metadata: &Value) -> Vec<String> {
    let mut named: Vec<String> = member_packages(metadata).map(package_name).collect();
    named.sort();
    named
}

/// The packages `metadata` describes that this workspace owns, in the order the
/// document lists them.
///
/// The membership test is the `workspace_members` id list, never the `packages`
/// array itself: `packages` is the whole resolved dependency graph, and a large
/// share of those third-party crates declare this workspace's own expression —
/// so a walk over all of them reports agreement it never established, about
/// crates nobody here can change.
fn member_packages(metadata: &Value) -> impl Iterator<Item = &Value> {
    let members: BTreeSet<&str> = named_array(metadata, "workspace_members")
        .filter_map(Value::as_str)
        .collect();
    named_array(metadata, "packages").filter(move |package| {
        package
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| members.contains(id))
    })
}

/// The array `metadata` holds under `key`, or nothing where it holds no array
/// there.
fn named_array<'a>(metadata: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    metadata
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// Whether every workspace member in `metadata` resolves `expression`.
///
/// `expression` is what the workspace declares, passed in rather than inferred:
/// a virtual manifest's `[workspace.package] license` is not a package of its
/// own in the metadata document, and a majority vote among members would call a
/// six-to-four split agreement.
///
/// A member declaring nothing is a divergence, never a match — "every member
/// declares the same thing" is satisfied by a member that declares nothing at
/// all, and that is the vacuity this check exists to close.
#[must_use]
pub fn member_licenses(metadata: &Value, expression: &str) -> Members {
    let mut declaring = Vec::new();
    let mut diverging = Vec::new();
    for package in member_packages(metadata) {
        let member = package_name(package);
        match declared_expression(package) {
            Declared::Expression(declared) if declared == expression => declaring.push(member),
            declared => diverging.push(DivergingMember { member, declared }),
        }
    }

    if diverging.is_empty() {
        declaring.sort();
        Members::AllDeclaring(declaring)
    } else {
        diverging.sort_by(|left, right| left.member.cmp(&right.member));
        Members::Diverging(diverging)
    }
}

/// The licence files `readme` links, and whether `root` holds them.
///
/// Takes the README's contents and the root as arguments, so the shipped README
/// and every fixture enter one code path.
///
/// A target is a licence-file link when its **final path segment begins with
/// `LICENSE`**, in either the `[text](TARGET)` or the angle-bracket
/// `[text](<TARGET>)` form. The **whole file** is read, not the `## License`
/// section's body: an under-collecting detector reports a refusal for a README
/// that does link a licence, and an all-clear for one whose link is dead.
///
/// # Errors
///
/// Returns the I/O failure when the root cannot be read. A link naming a file
/// that is not there is a verdict, not an error.
pub fn license_links(readme: &str, root: &Path) -> Result<Links, Box<dyn Error>> {
    let linked: BTreeSet<String> = link_targets(readme)
        .filter(|target| names_a_license_file(target))
        .collect();
    if linked.is_empty() {
        return Ok(Links::NoneLinked);
    }

    let mut unresolved = Vec::new();
    for target in &linked {
        // Joined, never canonicalised, for the reason `MissingFile::path`
        // records: a canonicalised path differs from the caller's own root on
        // Windows for reasons that have nothing to do with licences.
        if !root.join(target).try_exists()? {
            unresolved.push(target.clone());
        }
    }

    if unresolved.is_empty() {
        Ok(Links::AllResolve(linked.into_iter().collect()))
    } else {
        Ok(Links::Unresolved(unresolved))
    }
}

/// Every markdown link target in `readme`, in the order it wrote them, in
/// either link form.
///
/// The whole file, not the `## License` section's body — an under-collecting
/// scan reports a refusal for a README that does link a licence, and an
/// all-clear for one whose link is dead.
fn link_targets(readme: &str) -> impl Iterator<Item = String> {
    readme.split("](").skip(1).map(|after| {
        let target = after.split(')').next().unwrap_or(after).trim();
        unbracketed(target).to_owned()
    })
}

/// `target` with the angle brackets of the `[text](<TARGET>)` form removed, or
/// as written where it carries none.
fn unbracketed(target: &str) -> &str {
    target
        .strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(target)
}

/// Whether `target` names a licence file: its **final path segment** begins
/// with `LICENSE`.
///
/// The final segment rather than the whole string, so a target under a
/// directory is still recognised. Nothing grades that — see the module
/// header — and it is implemented as the spec defines it regardless.
fn names_a_license_file(target: &str) -> bool {
    target
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(target)
        .starts_with(LICENSE_PREFIX)
}

/// What the name of a licence file begins with.
const LICENSE_PREFIX: &str = "LICENSE";
