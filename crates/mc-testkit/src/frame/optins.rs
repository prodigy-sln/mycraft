//! The two environment opt-ins, read once and carried as a value.
//!
//! The variables are looked up through an injectable function, and
//! [`OptIns::from_environment`] is the only place in this crate that names
//! `std::env`. That isolates ambient state more strongly than a trait would: a
//! trait leaves every module free to ask for `"MYCRAFT_ALOW_NO_GPU"` with a
//! typo, whereas here the two names exist in exactly one function.
//!
//! It is also what makes the opt-ins testable at all. `std::env::set_var` is
//! `unsafe` in edition 2024 and `unsafe_code` is warned under `-D warnings`, so
//! a test that sets a variable needs an `#[allow(unsafe_code)]` — precisely the
//! escape hatch the quality gate exists to make visible. Tests inject a lookup
//! or construct the value directly instead.

use std::ffi::OsString;

/// Downgrades "no usable adapter" from a failure to an announced skip.
pub(crate) const ALLOW_NO_GPU: &str = "MYCRAFT_ALLOW_NO_GPU";

/// Permits writing golden files, which never happens otherwise.
pub(crate) const UPDATE_GOLDENS: &str = "MYCRAFT_UPDATE_GOLDENS";

/// Which of the harness's two opt-ins the caller has asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OptIns {
    pub allow_no_gpu: bool,
    pub update_goldens: bool,
}

impl OptIns {
    /// Reads both opt-ins through `lookup`.
    ///
    /// **Presence, not value, enables an opt-in**: `MYCRAFT_ALLOW_NO_GPU=0`
    /// still asks for the skip. The spec says "set" and "unset" and never
    /// mentions a value, and a variable someone bothered to set is a request.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<OsString>) -> Self {
        Self {
            allow_no_gpu: lookup(ALLOW_NO_GPU).is_some(),
            update_goldens: lookup(UPDATE_GOLDENS).is_some(),
        }
    }

    /// Reads both opt-ins from the process environment.
    #[must_use]
    pub fn from_environment() -> Self {
        Self::from_lookup(environment_lookup)
    }
}

/// Reads one variable from the process environment.
///
/// The only function in this crate that names `std::env`. It exists as a named
/// function rather than as `from_lookup(std::env::var_os)` because `var_os` is
/// generic over its key type, so passing it directly binds one concrete
/// lifetime and cannot satisfy `from_lookup`'s higher-ranked bound.
fn environment_lookup(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}
