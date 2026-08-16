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
/// **The textures are placeholders and they do not look like the blocks they are
/// on.** Stone and dirt are teal, grass is tan. That is correct: this increment
/// asks the textures to be deterministic, distinguishable and non-flat, and never
/// to be plausible — and correcting a colour per block name would be block content
/// hardcoded in the engine, which is the one thing the base game is not allowed to
/// be. Real textures are content, and content arrives as content.
const PALETTE_NOTICE: &str = "\
mycraft: the block textures in this build are placeholders — stone and dirt draw teal and grass
         draws tan. That is expected, not a fault: the textures are generated to be
         deterministic and distinguishable, not lifelike. Real artwork ships as content.";

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
    let root = match startup::content_root() {
        Ok(root) => root,
        Err(failure) => return Ending::failed(&failure, &failure.way_out()),
    };
    // The command line is read here rather than where its answer is spent, so
    // that the one place this process looks at its own arguments is the one
    // place it is started from. The save is located here for the same reason:
    // both are inputs the process's own environment supplies, and the worker
    // below is handed answers rather than the means to go looking for them.
    let preparation = launch::spawn_preparation(
        root,
        launch::save_path(),
        startup::acceptance_from(std::env::args()),
    );

    match gpu_startup::open() {
        Ok(gpu) => events::run(gpu, preparation),
        Err(ending) => ending,
    }
}
