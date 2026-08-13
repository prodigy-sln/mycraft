//! A fixture, not production code, and never compiled.
//!
//! It is a `.rs` file under `tests/fixtures/`, which cargo builds no target
//! from — only `tests/*.rs` and `tests/*/main.rs` become test binaries — so this
//! file exists solely to be *read* by the scan in `tests/intent_shape.rs`.
//!
//! It is the positive control for that scan pointed at an **enum**, and it is
//! shaped to fail it in three directions at once.
//!
//! The offending field sits in the **last** variant, behind a variant that
//! carries braces of its own. A scan that took everything up to the first `}` it
//! met would stop inside `Place`, read one innocent field and report nothing —
//! which looks exactly like a clean pass over the real declaration. Only a scan
//! that counts brace depth reaches `Mine`.
//!
//! The offending field is also a real one rather than a whole variant: an action
//! request may name *what* to place, and `block` here is correctly not flagged,
//! so the control also says the scan is discriminating rather than allergic to
//! variants with data.
//!
//! And `EditReport` beside it declares a cell the scan must pass over, because
//! the server's own answer legitimately names one — so a scan that merely
//! searched the file's text for the word, and would therefore report the real
//! module for the report it publishes, is caught as well.

pub enum ActionIntent {
    Break,
    Place { block: String },
    Mine { target_cell: [i32; 3] },
}

pub struct EditReport {
    pub cell: [i32; 3],
    pub from: String,
    pub to: String,
}
