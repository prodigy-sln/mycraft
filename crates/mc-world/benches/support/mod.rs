//! The fixtures the meshing benchmark runs against, the independent visible-face
//! oracle that judges them, and the budget verdict.
//!
//! It sits under `benches/` rather than beside the other test helpers because the
//! specification commits the fixtures *with* the benchmark. A test reaches the
//! same code with `#[path = "../benches/support/mod.rs"] mod support;` — the
//! sibling-`#[path]` convention this project already uses, pointed one directory
//! sideways. That is what lets the oracle and the verdict be exercised by ordinary
//! tests while they live beside the benchmark they serve, without adding a public
//! item to `mc-world`.
//!
//! Nothing here vouches for itself by coverage: `#[path]`-included code under
//! `benches/` is outside the coverage denominator. What stands in for it is
//! derivation — no expected quantity in this module may be a snapshot of the
//! mesher's own output.

// Each consumer links the whole module and uses a subset of it, so without this
// every consumer that skips a function would turn the lint stage red.
#![allow(dead_code)]

pub mod budget;
pub mod fixtures;
pub mod oracle;
