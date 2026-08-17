//! A careless or hostile declaration cannot take the loader down, and no
//! metamethod of a declaration's own runs while the loader reads it.
//!
//! A declaration file is the first production path that runs mod-authored code,
//! so the rule that there is no unbudgeted, unsandboxed entry into script
//! applies here before any callback exists. None of the mechanism is new: the
//! budget, the memory cap, the sandbox and the frozen environment all belong to
//! the scripting host, and raw field reads are already how the host reads a
//! table a mod handed it.
//!
//! **What these tests assert is the wiring, and that is the point.** A loader
//! that read a file and evaluated it round the side of the host would satisfy
//! every scenario about fields and every scenario about refusals, and would hang
//! the server on the first declaration that looped. Each test below therefore
//! goes through the loader rather than against a host built here, which would be
//! agreement between two copies of one decision.
//!
//! # One limit masks another, so each fixture is checked against both
//!
//! Filling a megabyte costs far more interrupt ticks than a small budget allows,
//! so a memory bomb under a small budget dies of ticks and reports the wrong
//! limit while passing. The two tests that trip a limit therefore assert which
//! limit was named **and which was not**, and the memory one asserts the
//! shipped cap by its own number: a loader that gave itself a smaller cap names
//! a different figure and reddens.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Attribution, QUARTZ, amber_after, attribution_of, blaming, fault_from,
    naming_the_file_alone, raw_field, read_reporting_print, registered, registry_from, text_field,
};
use mc_core::block::BlockRegistry;
use mc_script::HostLimits;
use mc_world::content::{LuauFileDefinitionSource, Printed};
use tempfile::TempDir;

/// A declaration whose top level never returns.
const A_LOOP_THAT_NEVER_RETURNS: &str = "while true do end\n";

/// A declaration that allocates far past anything one entry may hold.
///
/// Each appended string carries the loop index so that every one of them is
/// distinct and therefore separately allocated: the backend interns strings, and
/// without the index a thousand appends are a thousand references to one string
/// that no cap can stop, which would make this fixture unpassable by any
/// implementation and unfailable by a broken one.
///
/// A thousand appends of 4 KiB is about 4 MiB against the 256 KiB an entry may
/// add — sixteen times over, and a fraction of the absolute backstop, so the cap
/// is what stops it and it is stopped a long way before the machine notices.
/// Measured under the shipped limits: stopped for allocation in under a
/// millisecond.
const AN_ALLOCATION_PAST_THE_CAP: &str = "local held = {}\n\
     for index = 1, 1024 do held[index] = string.rep('x', 4096) .. index end\n";

/// A declaration reaching for the wall clock, which the sandbox removed.
const A_READING_OF_THE_CLOCK: &str = "local stamp = os.time()\n";

/// A declaration writing a name into the environment every other chunk on the
/// server reads through.
const AN_ASSIGNMENT_TO_A_GLOBAL: &str = "smuggled = true\n";

/// What the printing fixtures print.
///
/// Stated here and formatted into the chunk, so the expectation belongs to this
/// file rather than being a transcript of a run.
const A_LINE_THE_DECLARATION_PRINTS: &str = "the amber declaration is being read";

/// A declaration that prints once at its top level and then declares a block.
fn a_declaration_that_prints() -> String {
    amber_after(&format!("print('{A_LINE_THE_DECLARATION_PRINTS}')\n"))
}

/// A declaration carrying a metatable that offers a field the loader never
/// reads.
fn amber_under_an_unread_metatable() -> String {
    format!(
        "local declaration = {{\n\
         \t{},\n\
         \t{},\n\
         \t{},\n\
         }}\n\
         return setmetatable(declaration, {{ __index = {{ hardness = 3 }} }})\n",
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "true")
    )
}

/// A declaration stating no solidity of its own and supplying one through a
/// metatable.
fn amber_borrowing_its_solidity() -> String {
    format!(
        "local supplied = {{ solid = true }}\n\
         local declaration = {{\n\
         \t{},\n\
         \t{},\n\
         }}\n\
         return setmetatable(declaration, {{ __index = supplied }})\n",
        text_field("name", AMBER),
        text_field("texture", QUARTZ)
    )
}

/// A declaration whose metatable prints on every access it is asked to answer.
///
/// The metamethod answers a key the table does not hold, which is the only way
/// it can be reached at all — so a loader reading fields the ordinary way prints
/// once per missing field, and a loader reading raw prints nothing.
fn amber_printing_on_every_access() -> String {
    format!(
        "local declaration = {{\n\
         \t{},\n\
         \t{},\n\
         }}\n\
         return setmetatable(declaration, {{\n\
         \t__index = function(_, key)\n\
         \t\tprint('{A_LINE_THE_DECLARATION_PRINTS} ' .. tostring(key))\n\
         \t\treturn true\n\
         \tend,\n\
         }})\n",
        text_field("name", AMBER),
        text_field("texture", QUARTZ)
    )
}

/// A root holding one declaration file with `chunk` in it.
fn root_holding(directory: &TempDir, chunk: String) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, chunk)])
}

/// How many lines a declaration prints to run past what the host will keep, and
/// how many to stay comfortably inside it.
///
/// The first is stated as a count of lines this file's own rendering fixes the
/// width of, so the two numbers together say "far past" and "well inside"
/// without either being a figure copied out of a run. Two thousand lines of a
/// quarter of a kilobyte is half a megabyte of output against a shipped
/// allowance a quarter that size, and it costs a few thousand interrupt ticks
/// against a budget of a million.
const LINES_PAST_WHAT_IS_KEPT: usize = 2048;
const LINES_WELL_INSIDE_WHAT_IS_KEPT: usize = 3;

/// How much padding each printed line carries beyond its number.
const PADDING_PER_LINE: usize = 250;

/// The `position`-th line the printing fixtures print.
///
/// Numbered and zero-padded, so every line is distinct and all of them are the
/// same length: keeping the earliest and keeping the latest are then different
/// answers, which a fixture printing one line over and over could never
/// separate.
fn printed_line(position: usize) -> String {
    format!("{position:05} {}", "x".repeat(PADDING_PER_LINE))
}

/// A declaration printing `lines` of them at its top level and then declaring a
/// block.
///
/// The rendering is stated twice — here in Luau and above in Rust — rather than
/// one being derived from the other, because a fixture that asked the loader
/// what it had printed would agree with it whatever it answered.
fn a_declaration_printing(lines: usize) -> String {
    amber_after(&format!(
        "for index = 1, {lines} do\n\
         \tprint(string.format('%05d ', index) .. string.rep('x', {PADDING_PER_LINE}))\n\
         end\n"
    ))
}

/// What a record of what content printed says, reduced to the three facts this
/// suite is about.
///
/// A record rather than three separate assertions, so one comparison reports
/// every one of them at once and a loader that got two right is not mistaken for
/// one that got them all right.
#[derive(Debug, PartialEq, Eq)]
struct Summarised {
    /// The first line the record kept, or nothing where it kept none.
    earliest: Option<String>,
    /// Every line the chunk printed, counted: the ones kept, plus the ones the
    /// record says were not.
    accounted_for: usize,
    /// Whether the record says the host stopped keeping what was printed.
    stopped_keeping: bool,
}

/// What `printed` says, on those three terms.
fn summarised(printed: &Printed) -> Summarised {
    let (kept, dropped, stopped_keeping) = match printed {
        Printed::Whole(kept) => (kept, 0, false),
        Printed::Truncated { kept, dropped } => (kept, dropped.get(), true),
    };
    Summarised {
        earliest: kept.first().cloned(),
        accounted_for: kept.len() + usize::try_from(dropped).unwrap_or(usize::MAX),
        stopped_keeping,
    }
}

/// Which of the host's two limits a refusal named.
///
/// Both are asserted every time, because a fixture that trips the limit it was
/// not written for passes a one-sided check while measuring nothing.
#[derive(Debug, PartialEq, Eq)]
struct LimitNamed {
    attribution: Attribution,
    the_memory_cap: bool,
    the_call_and_loop_budget: bool,
}

/// What the refusal for `root` named, judged against the cap the engine ships.
fn limit_named_for(root: &Path) -> Result<LimitNamed, Box<dyn Error>> {
    let fault = fault_from(root)?;
    let shipped_cap = HostLimits::default().memory_cap.get();
    Ok(LimitNamed {
        attribution: attribution_of(&fault, AMBER_FILE),
        the_memory_cap: fault.cause.contains("allocation refused")
            && fault.cause.contains(&shipped_cap.to_string()),
        the_call_and_loop_budget: fault.cause.contains("call and loop budget exhausted"),
    })
}

#[test]
fn a_declaration_that_loops_without_returning_is_refused_naming_the_budget() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_after(A_LOOP_THAT_NEVER_RETURNS))?;

    let named = limit_named_for(&root)?;

    assert_eq!(
        named,
        LimitNamed {
            attribution: naming_the_file_alone(),
            the_memory_cap: false,
            the_call_and_loop_budget: true,
        },
        "the declaration this fixture wraps would register perfectly well; the loop in front of \
         it is the only thing wrong with it. A loader that evaluated round the side of the host \
         does not fail this test — it never finishes it, and takes the run with it, which is the \
         failure mode this whole file exists to keep out of the server"
    );
    Ok(())
}

#[test]
fn a_declaration_that_allocates_past_the_memory_cap_is_refused_naming_the_cap_and_not_the_budget()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_after(AN_ALLOCATION_PAST_THE_CAP))?;

    let named = limit_named_for(&root)?;

    assert_eq!(
        named,
        LimitNamed {
            attribution: naming_the_file_alone(),
            the_memory_cap: true,
            the_call_and_loop_budget: false,
        },
        "an operator reading this refusal has to be told which limit the mod hit, and the two \
         are easy to confuse: under a small enough budget this same bomb is stopped for spending \
         its ticks and reports the wrong limit while passing. The cap is asserted by its own \
         byte count as the engine ships it, so a loader that quietly gave itself a smaller one \
         names a different number here"
    );
    Ok(())
}

#[test]
fn a_declaration_that_reaches_for_the_clock_is_refused_naming_its_file() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_after(A_READING_OF_THE_CLOCK))?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        naming_the_file_alone(),
        "a declaration is evaluated in the sandboxed environment, which is the same environment \
         every other entry into script gets — the clock is not in it. Everything else in this \
         file would register, so a host whose sandbox did not reach chunk evaluation registers \
         the block and this goes red: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_declaration_that_assigns_a_global_is_refused_naming_its_file() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_after(AN_ASSIGNMENT_TO_A_GLOBAL))?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        naming_the_file_alone(),
        "the environment a chunk is evaluated against is frozen, so a declaration cannot leave a \
         name behind for the declarations read after it. One mod planting a global that every \
         later chunk reads is the shape this refusal exists for: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_declaration_carrying_a_metatable_the_loader_never_reads_registers_its_own_fields() -> TestResult
{
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_under_an_unread_metatable())?;

    let registry = registry_from(&root)?;

    assert_eq!(
        registered(&registry, AMBER)?,
        format!("textured {QUARTZ}, solid true"),
        "a metatable is an ordinary thing for a declaration to carry, and carrying one is not by \
         itself a reason to refuse a block. This is the control for the two tests around it: \
         without it, a loader that refused every table with a metatable would satisfy both of \
         them"
    );
    Ok(())
}

#[test]
fn a_solidity_supplied_only_through_a_metatable_is_refused_naming_that_field() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_borrowing_its_solidity())?;

    let fault = fault_from(&root)?;

    assert_eq!(
        attribution_of(&fault, AMBER_FILE),
        blaming(AMBER, "solid"),
        "a field the loader reads is a field the declaration states in its own right. Reading \
         through a metatable means running the mod's code on the host's schedule, outside every \
         budget, at a moment the mod chose — and then believing what it returns: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_metatable_that_prints_on_every_access_prints_nothing_while_the_root_is_read() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, amber_printing_on_every_access())?;

    let (printed, registered_anything) = read_reporting_print(&root)?;

    assert_eq!(
        (printed, registered_anything),
        (Printed::Whole(Vec::new()), false),
        "the metamethod here answers every key the table does not hold, so a loader reading \
         fields the ordinary way prints once per read and this record is not empty. The second \
         half is what stops the first from passing for the wrong reason: a loader that never \
         evaluated the chunk at all also prints nothing, and it would register the block it \
         never read"
    );
    Ok(())
}

#[test]
fn a_declaration_that_prints_at_its_top_level_is_recorded_as_having_printed_it() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, a_declaration_that_prints())?;

    let (printed, registered_anything) = read_reporting_print(&root)?;

    assert_eq!(
        (printed, registered_anything),
        (
            Printed::Whole(vec![A_LINE_THE_DECLARATION_PRINTS.to_owned()]),
            true
        ),
        "the control for every assertion in this suite that nothing was printed. A source that \
         recorded nothing whatever satisfies all of those forever, and the truncation counter a \
         later phase adds reads exactly like a chunk that printed nothing — so what content \
         printed has to be shown to arrive somewhere at least once"
    );
    Ok(())
}

#[test]
fn a_source_read_twice_reports_what_the_second_read_printed_and_not_both() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_holding(&directory, a_declaration_that_prints())?;
    let source = LuauFileDefinitionSource::new(&root);

    let mut first = BlockRegistry::new();
    first.apply(&source)?;
    let mut second = BlockRegistry::new();
    second.apply(&source)?;

    assert_eq!(
        source.printed(),
        Printed::Whole(vec![A_LINE_THE_DECLARATION_PRINTS.to_owned()]),
        "what a source printed is a record of the last time it was read, not a tally kept across \
         every read of its life. Hot reload re-reads a content root as often as an author saves \
         a file, and a record that accumulated would grow without bound while reporting, each \
         time, that the mod printed more than it did"
    );
    Ok(())
}

/// The distinction the host draws survives the loader's own boundary.
///
/// The host stops keeping script output at a stated allowance and says how many
/// lines it stopped keeping, because "the mod printed nothing" and "the host
/// stopped keeping what the mod printed" are different facts. A loader that
/// handed on the lines and dropped the count would re-open that at the boundary
/// a mod author's failed load is actually reported from — the record would read
/// as a chunk that printed exactly that much and stopped, which is the same
/// absence-that-reads-as-agreement one level up.
///
/// **Two roots, side by side, for the reason the host's own test runs two
/// hosts.** A record asserted on its own is satisfied by a loader that reports
/// truncation always, and — the other way — by one that reports it never; the
/// noisy root rejects the second and the quiet root rejects the first. A loader
/// that recorded nothing at all fails the quiet half too.
///
/// **The shipped allowance is what runs, and nothing here configures a limit.**
/// The loader gives itself the host's shipped limits by design, so the noisy
/// declaration is written to print far past the retained-output allowance while
/// costing a few thousand interrupt ticks against a budget of a million and
/// building each line well inside the per-entry memory cap. One limit masks
/// another: a fixture that came near either of those would have the chunk
/// stopped for a reason this test is not about, and would report an allowance
/// that was never reached.
///
/// **Every printed line is either kept or counted**, and that identity is what
/// the reading below is built on rather than a number copied out of a run. A
/// loader that kept the last lines instead of the first answers a different
/// earliest line; one that lost count of what it dropped fails the sum.
#[test]
fn a_declaration_that_printed_past_what_is_kept_is_recorded_differently_from_one_that_printed_little()
-> TestResult {
    let (loud, hushed) = (TempDir::new()?, TempDir::new()?);
    let noisy = root_holding(&loud, a_declaration_printing(LINES_PAST_WHAT_IS_KEPT))?;
    let quiet = root_holding(
        &hushed,
        a_declaration_printing(LINES_WELL_INSIDE_WHAT_IS_KEPT),
    )?;

    let (from_noisy, _) = read_reporting_print(&noisy)?;
    let (from_quiet, _) = read_reporting_print(&quiet)?;

    assert_eq!(
        (summarised(&from_noisy), summarised(&from_quiet)),
        (
            Summarised {
                earliest: Some(printed_line(1)),
                accounted_for: LINES_PAST_WHAT_IS_KEPT,
                stopped_keeping: true,
            },
            Summarised {
                earliest: Some(printed_line(1)),
                accounted_for: LINES_WELL_INSIDE_WHAT_IS_KEPT,
                stopped_keeping: false,
            }
        ),
        "a mod author whose load failed reads what their declaration printed on the way down, and \
         the first lines are the ones that locate the problem. What they must never read is a \
         record that stops without saying it stopped: `{}` lines were printed here and a record \
         claiming to hold all of them, or holding fewer and saying nothing about it, sends \
         somebody looking for output that was written and thrown away",
        LINES_PAST_WHAT_IS_KEPT
    );
    Ok(())
}
