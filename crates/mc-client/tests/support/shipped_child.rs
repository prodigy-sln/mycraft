//! Running the shipped executable as a child and reading what it wrote, bounded.
//!
//! # Why this is a module of its own
//!
//! `tests/shipped_binary.rs` does two separable things: it runs the built client
//! through the process boundary, and it grades what came back against what the
//! product is supposed to say. The *running* is what grows — a second notice to
//! wait for, a third argument to pass — and the grading is the guard. The split
//! follows that seam, which is `support/printed_refusals.rs`'s own reason for
//! existing rather than a line count.
//!
//! # Bounded, never `output()`, and the reason is measured
//!
//! A launch that succeeds opens a window and runs until somebody closes it, so
//! `Command::output()` waits for a child that will never end — measured at **606
//! seconds** under exactly the mutation those readings exist for, with the run
//! killed by hand. Everything here is bounded by [`PATIENCE`] and reports the
//! difference between a child that ended and one that had to be killed, because a
//! hang gets blamed on the machine and a red names the mechanism.
//!
//! # Standard output is never piped
//!
//! A pipe nobody drains is a child that blocks once it fills, which would be the
//! reading's own hang. The one reading that grades standard output uses
//! `output()`, which drains both — and it can, because that path refuses before it
//! opens a device.

// Each binary that includes this drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// How long a child is given to say a line before the reading gives up.
///
/// Generous against its real cost — the child reads a handful of block
/// declarations and a 65 KB save before it says anything — because this bound
/// exists to turn a missing line into a failure rather than to measure anything.
/// A run that wedges gets blamed on the machine; a run that fails names the
/// defect.
pub const PATIENCE: Duration = Duration::from_secs(20);

/// How a subprocess ended.
///
/// Three-valued rather than a boolean: a process killed by a signal carries no
/// status at all, and that must not read as "it refused".
#[derive(Debug, PartialEq, Eq)]
pub enum Exited {
    /// Successfully.
    Zero,
    /// With a failing status, whichever one — the mapping from ending to status
    /// is graded where it lives and is not this test's subject.
    NonZero,
    /// Carrying no status at all.
    WithoutACode,
}

/// Whether a child finished inside the reading's patience.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// It ended by itself, with this status.
    OnItsOwn(Exited),
    /// It was still running when the deadline passed, and was killed.
    HadToBeKilled,
}

/// Everything a run of the binary wrote to its error stream, and how it ended.
#[derive(Debug)]
pub struct Said {
    pub text: String,
    pub ended: Ended,
}

/// Runs the built client in `game`, optionally with `argument`, and reads its error
/// stream until the child ends or [`PATIENCE`] runs out.
///
/// # Errors
///
/// Returns an error if the child cannot be spawned, its error stream cannot be taken,
/// or it cannot be waited on.
pub fn what_the_binary_said(game: &Path, argument: Option<&str>) -> Result<Said, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mc-client"));
    command
        .current_dir(game)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(argument) = argument {
        command.arg(argument);
    }
    let mut child = command.spawn()?;
    let lines = lines_of(&mut child)?;
    let (everything, ended_on_its_own) = until_it_stopped(&lines);
    let ended = if ended_on_its_own {
        Ended::OnItsOwn(exit_of(&child.wait()?))
    } else {
        drop(child.kill());
        drop(child.wait());
        Ended::HadToBeKilled
    };
    Ok(Said {
        text: line_by_line(&everything),
        ended,
    })
}

/// Runs the built client in `game` and reads its error stream until it says a line
/// containing `clause`, then kills it.
///
/// Hands back what it was looking for and **everything it read on the way**, so a
/// run that never said it fails with the child's own output in the message rather
/// than with an empty absence.
///
/// # Errors
///
/// Returns an error if the child cannot be spawned or its error stream cannot be
/// taken.
pub fn the_line_the_binary_wrote(
    game: &Path,
    clause: &str,
) -> Result<(Option<String>, Vec<String>), Box<dyn Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mc-client"))
        .current_dir(game)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let lines = lines_of(&mut child)?;
    let read = until_it_said(&lines, clause);
    // Killed rather than waited out: this launch succeeds, and a client whose
    // launch succeeds goes on to open a window and take the pointer. Both answers
    // are dropped deliberately — a child that has already gone is the outcome this
    // asks for, and a failure to reap it says nothing about the line.
    drop(child.kill());
    drop(child.wait());
    Ok(read)
}

/// How `status` ended, without pinning which failing code it chose.
pub fn exit_of(status: &ExitStatus) -> Exited {
    match status.code() {
        Some(0) => Exited::Zero,
        Some(_) => Exited::NonZero,
        None => Exited::WithoutACode,
    }
}

/// The child's error stream, one line at a time, off a thread so that reading it
/// can be given up on.
///
/// # Errors
///
/// Returns an error if the stream was already taken.
fn lines_of(child: &mut Child) -> Result<Receiver<String>, Box<dyn Error>> {
    let stream = child
        .stderr
        .take()
        .ok_or("the child was spawned without an error stream to read")?;
    let (send, receive) = mpsc::channel();
    // Stops of its own accord when the reader hangs up, which is what the `take_while`
    // is: a send into a dropped receiver is the end of this thread's work.
    std::thread::spawn(move || {
        BufReader::new(stream)
            .lines()
            .map_while(Result::ok)
            .take_while(|line| send.send(line.clone()).is_ok())
            .for_each(drop);
    });
    Ok(receive)
}

/// Every line the child wrote, and whether its stream ended before the deadline.
///
/// A stream that ends is a child that has exited or closed it; a deadline that passes
/// is a child still running. Those are different answers and the caller needs both.
fn until_it_stopped(lines: &Receiver<String>) -> (Vec<String>, bool) {
    let deadline = Instant::now() + PATIENCE;
    let mut everything = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match lines.recv_timeout(left) {
            Ok(line) => everything.push(line),
            Err(RecvTimeoutError::Disconnected) => return (everything, true),
            Err(RecvTimeoutError::Timeout) => return (everything, false),
        }
    }
}

/// Every line read until one contains `clause`, and that line where there was
/// one.
///
/// Bounded by [`PATIENCE`] rather than by the stream ending, because the stream
/// does not end: the child is still running when the line arrives.
fn until_it_said(lines: &Receiver<String>, clause: &str) -> (Option<String>, Vec<String>) {
    let deadline = Instant::now() + PATIENCE;
    let mut everything = Vec::new();
    let mut found = None;
    while found.is_none() {
        let left = deadline.saturating_duration_since(Instant::now());
        let Ok(line) = lines.recv_timeout(left) else {
            // Both ways of running out are the same answer: the line was not said.
            // A stream that ended is a child that exited without saying it, and a
            // deadline that passed is one still running and still silent.
            return (None, everything);
        };
        found = line.contains(clause).then(|| line.clone());
        everything.push(line);
    }
    (found, everything)
}

/// The lines put back together the way the child wrote them, so a whole-stream
/// comparison is a comparison of what a person reads.
fn line_by_line(everything: &[String]) -> String {
    everything
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<Vec<_>>()
        .concat()
}
