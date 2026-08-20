//! The binary: warn about the palette, run the client, hand the ending a stream
//! to be said on, return the status it says.
//!
//! **How a failure reads is not decided here, and that is deliberate.** A refusal
//! is the only thing a mod author with a broken content file ever gets, and this
//! crate is excluded from the coverage denominator wholesale — text composed here
//! is text nothing measures. So the wording, the chain walk and the prefix all
//! live beside the endings in `mc_render::window`, where a test can call them,
//! and what is left here is the choice of `stderr` and nothing else.
//!
//! Everything it runs lives in the library beside it, so the tests that shoot the
//! goldens import the same startup path this does — see `lib.rs` for why that
//! matters more than it looks.

use std::io;
use std::process::ExitCode;

use mc_client::{events, gpu_startup, launch, startup};

use mc_render::window::{Ending, exit_code, report};

/// What the player is told before the window opens.
///
/// **The shipped blocks draw baked art now, and a key nothing baked still
/// draws a generated stand-in.** That second half is the sentence worth
/// printing: a mod author's first block declares a texture key nobody has drawn
/// yet, and what they get is a deterministic, distinguishable, deliberately
/// implausible texture derived from the key itself rather than a refusal. Saying
/// so here is what stops the stand-in reading as a fault in the art build they
/// have just run.
const PALETTE_NOTICE: &str = "\
mycraft: blocks whose art has been baked draw it; a texture key nothing has baked yet draws a
         generated stand-in instead of refusing the launch. A stand-in is deterministic and
         distinguishable, never lifelike, and it means nothing is wrong.";

fn main() -> ExitCode {
    println!("{PALETTE_NOTICE}");
    let ending = run();
    // A client that cannot write to its own error stream has nowhere left to say
    // so, and the status the shell reads is the same either way.
    let _written = report(&ending, &mut io::stderr());
    ExitCode::from(exit_code(&ending))
}

/// Starts the replay preparing, opens a device, and runs the window until it
/// closes.
///
/// The device is opened **before** the window: a machine that cannot draw this
/// gets a message and a status, never a window that opens and then shows nothing.
fn run() -> Ending {
    let root = match startup::shipped_content() {
        Ok(root) => root,
        Err(failure) => return Ending::failed(&failure, &failure.way_out()),
    };
    // The command line is read here rather than where its answer is spent, so
    // that the one place this process looks at its own arguments is the one
    // place it is started from. The save is located here for the same reason:
    // both are inputs the process's own environment supplies, and the worker
    // below is handed answers rather than the means to go looking for them.
    //
    // The built set is judged on the way through, which is why this can refuse
    // before a device is opened: a contributor who has not run the art build
    // reads the command to run rather than waiting out a world they will not be
    // shown.
    let starting = match launch::start(
        root,
        launch::save_path(),
        startup::acceptance_from(std::env::args()),
    ) {
        Ok(starting) => starting,
        Err(failure) => return Ending::failed(&failure, &failure.way_out()),
    };

    match gpu_startup::open() {
        Ok(gpu) => events::run(gpu, starting),
        Err(ending) => ending,
    }
}
