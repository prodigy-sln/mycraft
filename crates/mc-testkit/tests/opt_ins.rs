//! Reading the two environment opt-ins without touching the environment.
//!
//! `std::env::set_var` is `unsafe` in edition 2024 and `unsafe_code` is warned
//! under `-D warnings`, so a test that sets a variable needs an
//! `#[allow(unsafe_code)]` — exactly the escape hatch the quality gate exists to
//! make visible. The lookup is injected instead, which also means the two
//! variable names live in exactly one place in the crate and cannot be
//! mistyped somewhere else.

use std::ffi::OsString;

use mc_testkit::frame::OptIns;

const ALLOW_NO_GPU: &str = "MYCRAFT_ALLOW_NO_GPU";
const UPDATE_GOLDENS: &str = "MYCRAFT_UPDATE_GOLDENS";
/// A value that looks like "off" to anyone reading it as a boolean. Presence is
/// what counts, so it must not be.
const FALSY: &str = "0";

/// A stand-in process environment in which exactly `present` is set, each to a
/// value that reads as false.
fn environment(present: &[&str]) -> impl Fn(&str) -> Option<OsString> {
    let present: Vec<String> = present.iter().map(|name| (*name).to_owned()).collect();
    move |name| {
        present
            .iter()
            .any(|set| set == name)
            .then(|| OsString::from(FALSY))
    }
}

#[test]
fn an_opt_in_set_to_a_falsy_value_is_still_enabled() {
    let opt_ins = OptIns::from_lookup(environment(&[ALLOW_NO_GPU]));

    assert!(
        opt_ins.allow_no_gpu,
        "the contract is presence, not value: setting it to `{FALSY}` still \
         asks for the skip"
    );
}

#[test]
fn each_variable_enables_only_its_own_opt_in() {
    let opt_ins = OptIns::from_lookup(environment(&[UPDATE_GOLDENS]));

    assert_eq!(
        opt_ins,
        OptIns {
            allow_no_gpu: false,
            update_goldens: true,
        },
        "permission to rewrite goldens is not permission to skip a capture"
    );
}
