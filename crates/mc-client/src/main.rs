//! The binary: run the client, hand the ending a stream to be said on, return
//! the status it says.
//!
//! **It says nothing about the art on its own account, and that is a repair.**
//! The sentence about generated stand-ins used to be printed here — before the
//! content root had been read and before any set had been judged — so it named no
//! key and read identically whether every declared key was covered or none was.
//! What replaced it is composed on the preparation worker, where the keys content
//! declares and the keys the built set covers are both in hand, and it names the
//! ones that had no image.
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

use mc_client::notice::Notices;
use mc_client::{events, gpu_startup, launch, startup};

use mc_render::window::{Ending, exit_code, report};

fn main() -> ExitCode {
    // The one place in this process that names a stream, and it names it once for
    // the notices and the ending alike.
    let notices = Notices::writing_to(Box::new(io::stderr()));
    let ending = run(&notices);
    // A client that cannot write to its own error stream has nowhere left to say
    // so, and the status the shell reads is the same either way.
    let _written = notices.with(|sink| report(&ending, sink));
    ExitCode::from(exit_code(&ending))
}

/// Starts the replay preparing, opens a device, and runs the window until it
/// closes.
///
/// The device is opened **before** the window: a machine that cannot draw this
/// gets a message and a status, never a window that opens and then shows nothing.
fn run(notices: &Notices) -> Ending {
    let root = match startup::shipped_content() {
        Ok(root) => root,
        Err(failure) => return Ending::failed(&failure),
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
        notices,
    ) {
        Ok(starting) => starting,
        Err(failure) => return Ending::failed(&failure),
    };

    match gpu_startup::open() {
        Ok(gpu) => events::run(gpu, starting, notices),
        Err(ending) => ending,
    }
}
