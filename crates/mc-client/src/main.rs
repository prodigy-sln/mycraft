//! The binary: warn about the palette, run the client, return what the ending
//! says.
//!
//! Everything it runs lives in the library beside it, so the tests that shoot the
//! goldens import the same startup path this does — see `lib.rs` for why that
//! matters more than it looks.

use std::process::ExitCode;

use mc_client::{events, gpu_startup, startup};

use mc_render::window::{Ending, exit_code};

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
    report(&ending);
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
        Err(failure) => {
            return Ending::Failed {
                report: failure.to_string(),
            };
        }
    };
    let preparation = startup::spawn_preparation(root);

    match gpu_startup::open() {
        Ok(gpu) => events::run(gpu, preparation),
        Err(ending) => ending,
    }
}

/// Says how the run ended, for every ending that is not simply the player closing
/// the window.
fn report(ending: &Ending) {
    match ending {
        Ending::Closed => {}
        Ending::Startup(failure) => eprintln!("mycraft: {failure}"),
        Ending::Frame(reason) => {
            eprintln!("mycraft: the run stopped because the graphics device was lost ({reason:?})");
        }
        Ending::Failed { report } => eprintln!("mycraft: {report}"),
    }
}
