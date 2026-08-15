//! The `voxforge` binary.
//!
//! Three lines, and deliberately so: every decision — argument parsing,
//! dispatch, rendered text, exit-code selection — lives in the library where a
//! test can reach it. A binary carrying any of that would earn the coverage
//! exclusion the binary crates have, and with it the blindness that exclusion
//! brings.

fn main() -> std::process::ExitCode {
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    voxforge::cli::run(std::env::args_os().collect(), &mut out, &mut err).into()
}
