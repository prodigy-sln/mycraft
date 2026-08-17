//! What the host keeps of everything content prints, and what it says when it
//! stops keeping it.
//!
//! # The buffer this bounds had no route content could reach until now
//!
//! `print` is a permitted global and every call hands the host a string it keeps
//! forever. Nothing in production evaluated a chunk before, so that buffer was an
//! observable for tests; the block loader is the first path a mod author can
//! reach it through. Each `print` costs one interrupt tick, so a single chunk can
//! make on the order of half a million calls inside its shipped budget, each
//! string built inside the per-entry memory cap and then becoming script-side
//! garbage while the host-side copy is retained **outside every limit that
//! exists**. Multiplied by a full content root that is tens of gigabytes, which
//! is one careless file taking down a server.
//!
//! # Three properties, and the third is the one that is easy to leave out
//!
//! **The bound lives with the other limits**, so an operator can read and set it
//! rather than find it as a constant inside the host.
//!
//! **Reaching it stops recording; it does not drop the oldest.** Whoever is
//! debugging a load wants the beginning of the story — the first line a chunk
//! printed is the one that locates the problem and the millionth is not. Every
//! line the fixtures below print is distinct, which is what separates keeping the
//! earliest from keeping the latest; a fixture printing one line over and over
//! reads identically under both.
//!
//! **Truncation is reported.** "The mod printed nothing" and "the host stopped
//! keeping what the mod printed" are different facts, and a record that cannot
//! tell them apart is an absence that reads as agreement. So the first test below
//! runs two hosts and compares them: one whose chunk printed far too much, one
//! whose chunk printed nothing at all. Asserting the truncated one alone would be
//! satisfied by a host that never recorded anything.
//!
//! # Two things the fixtures are shaped around
//!
//! **The bound is on one host's whole life, not on one entry.** The loader
//! evaluates thousands of chunks through a single host, so an allowance that
//! reset whenever the host collected what a chunk printed would bound each
//! declaration and nothing at all across a content root. One test drives two
//! chunks through one host for exactly that reason.
//!
//! **One limit masks another.** Only the retained-output bound is configured
//! here; the budget and the memory cap stay at the values that ship. A test that
//! shrank the budget to make a chunk cheap would find the chunk stopped for
//! exhausting its budget and would report a bound that had never been reached.

use std::error::Error;
use std::num::NonZeroUsize;

use mc_script::{HostLimits, ScriptHost, ScriptValue};

type TestResult = Result<(), Box<dyn Error>>;

/// How many lines the noisy fixture prints.
///
/// Small enough that printing them costs a few hundred interrupt ticks against
/// a shipped budget of a million: the subject is what the host keeps, and a
/// fixture that also came near the budget would leave it open which limit
/// answered.
const LINES_A_NOISY_CHUNK_PRINTS: usize = 64;

/// How many of those lines the configured bound has room for.
const LINES_THE_BOUND_ADMITS: usize = 20;

/// Bytes the bound carries beyond those lines, too few for one more.
///
/// The bound is deliberately **not** a whole number of lines. A host that filled
/// the remainder with the first few bytes of the next line would keep a
/// fragment of a line nobody printed, and a bound that is an exact multiple of
/// the line length can never see that.
const BYTES_TOO_FEW_FOR_ANOTHER_LINE: usize = 4;

/// A chunk that returns without printing anything.
const A_CHUNK_THAT_PRINTS_NOTHING: &str = "return true\n";

/// The `position`-th line a noisy chunk prints.
///
/// Every line is distinct, so keeping the earliest and keeping the latest are
/// different answers. Zero-padded so that all of them are the same length,
/// which is what lets the bound be stated as a count of lines without the
/// arithmetic depending on which lines happened to fit.
fn line(position: usize) -> String {
    format!("line {position:04}")
}

/// How many bytes one of those lines is.
///
/// Measured off the rendering above rather than written down, so that widening
/// the line moves every bound in this file with it.
fn line_bytes() -> usize {
    line(1).len()
}

/// A bound with room for `lines` whole lines and no more.
fn a_bound_holding(lines: usize) -> Result<NonZeroUsize, Box<dyn Error>> {
    NonZeroUsize::new(line_bytes() * lines).ok_or_else(|| "a bound of zero retains nothing".into())
}

/// A bound with room for `lines` whole lines and a remainder too small for
/// another.
fn a_bound_holding_a_little_over(lines: usize) -> Result<NonZeroUsize, Box<dyn Error>> {
    NonZeroUsize::new(line_bytes() * lines + BYTES_TOO_FEW_FOR_ANOTHER_LINE)
        .ok_or_else(|| "a bound of zero retains nothing".into())
}

/// The first `count` lines a noisy chunk prints.
fn the_first_lines(count: usize) -> Vec<String> {
    (1..=count).map(line).collect()
}

/// A chunk printing `lines` distinct lines and then returning.
fn a_chunk_printing(lines: usize) -> String {
    format!(
        "for index = 1, {lines} do\n\
         \tprint(string.format('line %04d', index))\n\
         end\n\
         return true\n"
    )
}

/// A host under the shipped limits but for what it will retain of script
/// output.
fn host_retaining(bytes: NonZeroUsize) -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        retained_print_bytes: bytes,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host would not start: {error}").into())
}

/// Evaluates `source` as `chunk`, failing the test if it did not run.
fn evaluated(host: &mut ScriptHost, chunk: &str, source: &str) -> Result<(), Box<dyn Error>> {
    match host.evaluate(chunk, source) {
        Ok(ScriptValue::Boolean(true)) => Ok(()),
        Ok(other) => Err(format!("`{chunk}` was written to return true, not {other:?}").into()),
        Err(fault) => Err(format!("`{chunk}` did not evaluate: {fault}").into()),
    }
}

/// What one host kept of what content printed, and what it says it did not
/// keep.
#[derive(Debug, PartialEq, Eq)]
struct Retained {
    kept: Vec<String>,
    lines_dropped: u64,
}

/// What `host` retained.
fn retained_by(host: &ScriptHost) -> Retained {
    Retained {
        kept: host.printed().to_vec(),
        lines_dropped: host.dropped_print_lines(),
    }
}

/// A host that kept everything it was given.
fn everything(lines: usize) -> Retained {
    Retained {
        kept: the_first_lines(lines),
        lines_dropped: 0,
    }
}

/// A host that stopped after `kept` lines, having been handed `printed` in all.
fn stopped_after(kept: usize, printed: usize) -> Result<Retained, Box<dyn Error>> {
    let dropped = printed
        .checked_sub(kept)
        .ok_or("a host cannot drop more lines than it was handed")?;
    Ok(Retained {
        kept: the_first_lines(kept),
        lines_dropped: u64::try_from(dropped)?,
    })
}

#[test]
fn output_past_what_a_host_retains_keeps_the_earliest_lines_and_reports_that_it_kept_no_more()
-> TestResult {
    let mut noisy = host_retaining(a_bound_holding_a_little_over(LINES_THE_BOUND_ADMITS)?)?;
    evaluated(
        &mut noisy,
        "amber.luau",
        &a_chunk_printing(LINES_A_NOISY_CHUNK_PRINTS),
    )?;
    let mut quiet = host_retaining(a_bound_holding_a_little_over(LINES_THE_BOUND_ADMITS)?)?;
    evaluated(&mut quiet, "quartz.luau", A_CHUNK_THAT_PRINTS_NOTHING)?;

    assert_eq!(
        (
            retained_by(&noisy),
            retained_by(&quiet),
            BYTES_TOO_FEW_FOR_ANOTHER_LINE < line_bytes(),
        ),
        (
            stopped_after(LINES_THE_BOUND_ADMITS, LINES_A_NOISY_CHUNK_PRINTS)?,
            everything(0),
            true,
        ),
        "three separate failures are rejected by this one comparison. A host that kept the \
         *latest* lines within the bound answers `line 0045` upwards, which is the end of a story \
         whose beginning located the problem. A host that filled its remaining four bytes with \
         the front of the twenty-first line answers a fragment nobody printed, which is why the \
         bound is deliberately not a whole number of lines. And a host that recorded nothing at \
         all satisfies every assertion about the truncated record on its own — which is what the \
         second host is for: `the mod printed nothing` and `the host stopped keeping what the mod \
         printed` are different facts, and a record that cannot tell them apart is an absence \
         that reads as agreement"
    );
    Ok(())
}

#[test]
fn output_that_exactly_fills_what_a_host_retains_is_kept_whole_and_nothing_is_reported_dropped()
-> TestResult {
    let mut host = host_retaining(a_bound_holding(LINES_A_NOISY_CHUNK_PRINTS)?)?;

    evaluated(
        &mut host,
        "amber.luau",
        &a_chunk_printing(LINES_A_NOISY_CHUNK_PRINTS),
    )?;

    assert_eq!(
        retained_by(&host),
        everything(LINES_A_NOISY_CHUNK_PRINTS),
        "the accepting side of this bound, which no scenario states: the other three bounds in \
         this feature each have one and this one had nothing. A bound stated only from the \
         truncating side leaves `>` and `>=` indistinguishable, and a counter that reported a \
         line dropped whenever output merely reached the allowance would make every full record \
         read as a truncated one — after which nobody could trust the distinction the truncating \
         test asserts"
    );
    Ok(())
}

#[test]
fn a_second_chunk_printing_after_the_bound_was_reached_adds_nothing_and_is_reported_dropped()
-> TestResult {
    let mut host = host_retaining(a_bound_holding(LINES_THE_BOUND_ADMITS)?)?;

    evaluated(
        &mut host,
        "amber.luau",
        &a_chunk_printing(LINES_THE_BOUND_ADMITS),
    )?;
    evaluated(
        &mut host,
        "quartz.luau",
        &a_chunk_printing(LINES_THE_BOUND_ADMITS),
    )?;

    assert_eq!(
        retained_by(&host),
        stopped_after(LINES_THE_BOUND_ADMITS, LINES_THE_BOUND_ADMITS * 2)?,
        "the bound is on one host's whole life and not on one entry into script, and this is the \
         only test that can tell those apart. The loader evaluates every declaration in a content \
         root through a single host, so an allowance that started again whenever the host \
         collected what a chunk had printed would bound one declaration and nothing whatever \
         across four thousand of them — which is the arithmetic that made this bound necessary in \
         the first place. The first chunk fills the allowance exactly, so every line of the \
         second is refused and counted"
    );
    Ok(())
}
