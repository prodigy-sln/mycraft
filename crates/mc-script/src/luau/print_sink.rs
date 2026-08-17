//! What the host keeps of everything content prints, and what it stopped
//! keeping.
//!
//! `print` is a permitted global and every call hands the host a string. Until a
//! production path evaluated a chunk, that buffer was an observable for tests and
//! nothing a mod author wrote could reach it. It is reachable now, and the
//! arithmetic is not close: each `print` is a host call costing one interrupt
//! tick, so one chunk can make on the order of half a million calls inside its
//! shipped budget, each string built inside the per-entry memory cap and then
//! becoming script-side garbage while the host-side copy is retained **outside
//! every limit that exists**. Across a full content root that is tens of
//! gigabytes — one careless file taking the server down through the loader.
//!
//! # Three properties, and the third is the one that is easy to leave out
//!
//! **The allowance is one host's whole life, not one entry into script.**
//! Collecting what a chunk printed does not hand the allowance back. The loader
//! drives every declaration in a content root through a single host, so an
//! allowance that reset on each drain would bound one declaration and nothing at
//! all across four thousand of them — which is the arithmetic that made this
//! bound necessary in the first place.
//!
//! **Reaching it stops recording rather than dropping the oldest**, and a line is
//! kept whole or not at all. Whoever is debugging a load wants the beginning of
//! the story: the first line a chunk printed is the one that locates the problem
//! and the millionth is not. Keeping a fragment of the line that did not fit
//! would put text in the record that nobody printed.
//!
//! **Truncation is counted.** "The mod printed nothing" and "the host stopped
//! keeping what the mod printed" are different facts, and a record that cannot
//! tell them apart is an absence that reads as agreement.

use std::num::NonZeroUsize;

/// What content printed, bounded, and a tally of what was refused.
#[derive(Debug)]
pub(crate) struct PrintSink {
    lines: Vec<String>,
    /// Bytes kept over this host's whole life. **Never reduced by a drain** —
    /// see the module note; this is what makes the allowance a property of the
    /// host rather than of one entry into script.
    kept: usize,
    /// What those bytes are measured against.
    allowance: usize,
    /// Lines the host was handed and did not keep.
    dropped: u64,
}

impl PrintSink {
    /// A sink that will keep `allowance` bytes of what content prints.
    pub(crate) fn new(allowance: NonZeroUsize) -> Self {
        Self {
            lines: Vec::new(),
            kept: 0,
            allowance: allowance.get(),
            dropped: 0,
        }
    }

    /// Keeps `line` if the allowance has room for the whole of it, and counts it
    /// as dropped if it does not.
    pub(crate) fn record(&mut self, line: String) {
        match self.kept.checked_add(line.len()) {
            Some(total) if total <= self.allowance => {
                self.kept = total;
                self.lines.push(line);
            }
            _ => self.dropped = self.dropped.saturating_add(1),
        }
    }

    /// Everything kept since the last collection, leaving the sink empty.
    ///
    /// The allowance is deliberately untouched.
    pub(crate) fn drain(&mut self) -> Vec<String> {
        self.lines.drain(..).collect()
    }

    /// How many lines the host was handed and did not keep.
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped
    }
}
