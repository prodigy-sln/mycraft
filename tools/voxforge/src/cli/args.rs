//! What the command line says.
//!
//! Parsing only — nothing here reads a file or renders anything. That split is
//! what lets a bad argument be refused in the same terms as a bad document,
//! through the same [`Fault`], rather than through whatever `clap` would have
//! said about a type nobody wrote.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::fault::{Fault, Origin};
use crate::format::{PartName, StateName};
use crate::volume::StateSelection;

/// Where materials are read from when nobody says otherwise.
const DEFAULT_MATERIALS: &str = "content/base/materials";

/// How many pixels a voxel spans when nobody says otherwise.
const DEFAULT_SCALE: u32 = 8;

/// The whole command line.
#[derive(Debug, Parser)]
#[command(
    name = "voxforge",
    about = "Author voxel models, and see what you made"
)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// The four things this tool does.
///
/// Each variant carries its own arguments as a struct rather than as inline
/// fields, so that a dispatcher stays a dispatcher: `texture` alone spells eight
/// arguments, and destructuring four subcommands in one match makes the one
/// function that chooses between them longer than the ones doing the work.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Render a model and write it as a PNG.
    Preview(PreviewArgs),
    /// Report what a model is, and what is wrong with it.
    Inspect(InspectArgs),
    /// Emit a model as a block texture, one face or all six.
    Texture(TextureArgs),
    /// Bake a manifest's entries into a named texture set.
    Build(BuildArgs),
}

/// What `preview` was asked for.
#[derive(Debug, Args)]
pub struct PreviewArgs {
    /// The `.mcvox` document to read.
    pub document: PathBuf,
    /// Where to write the image.
    #[arg(long)]
    pub out: PathBuf,
    /// Which view to render. Omitted, every view is tiled into one sheet.
    #[arg(long)]
    pub view: Option<String>,
    /// How many pixels one voxel spans.
    #[arg(long = "pixels-per-voxel")]
    pub pixels_per_voxel: Option<u32>,
    /// Where to resolve material keys from.
    #[arg(long)]
    pub materials: Option<PathBuf>,
    /// Which state a part takes, as `part=state`. Repeatable.
    #[arg(long = "state")]
    pub states: Vec<String>,
}

/// What `build` was asked for.
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// The manifest naming what to bake, and where it goes.
    pub document: PathBuf,
}

/// What `inspect` was asked for.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// The `.mcvox` document to read.
    pub document: PathBuf,
    /// Which state a part takes, as `part=state`. Repeatable.
    #[arg(long = "state")]
    pub states: Vec<String>,
}

/// What `texture` was asked for.
#[derive(Debug, Args)]
pub struct TextureArgs {
    /// The `.mcvox` document to read.
    pub document: PathBuf,
    /// The directory the images are written into, one file per face.
    #[arg(long)]
    pub out: PathBuf,
    /// Which face to emit.
    #[arg(long, group = "selection")]
    pub face: Option<String>,
    /// Emit a block's whole six faces from this one invocation.
    #[arg(long = "all-faces", group = "selection")]
    pub all_faces: bool,
    /// Refuse a texture that will not tile, rather than reporting it.
    #[arg(long)]
    pub seamless: bool,
    /// How many pixels one voxel spans.
    #[arg(long = "pixels-per-voxel")]
    pub pixels_per_voxel: Option<u32>,
    /// Where to resolve material keys from.
    #[arg(long)]
    pub materials: Option<PathBuf>,
    /// Which state a part takes, as `part=state`. Repeatable.
    #[arg(long = "state")]
    pub states: Vec<String>,
}

impl Command {
    /// The document this command is about.
    #[must_use]
    pub fn document(&self) -> &PathBuf {
        match self {
            Self::Preview(asked) => &asked.document,
            Self::Inspect(asked) => &asked.document,
            Self::Texture(asked) => &asked.document,
            Self::Build(asked) => &asked.document,
        }
    }

    /// The states this command selects.
    ///
    /// # Errors
    ///
    /// Returns a [`Fault`] naming the offending argument when one is not
    /// `part=state`.
    pub fn state_selection(&self) -> Result<StateSelection, Fault> {
        let origin = Origin::new(self.document());
        let mut chosen = StateSelection::default();
        for spelling in self.states() {
            let (part, state) = split_selection(spelling, &origin)?;
            chosen = chosen.with(PartName::new(part), StateName::new(state));
        }
        Ok(chosen)
    }

    /// Every `--state` argument this command carries, as written.
    fn states(&self) -> &[String] {
        match self {
            Self::Preview(asked) => &asked.states,
            Self::Inspect(asked) => &asked.states,
            Self::Texture(asked) => &asked.states,
            // A manifest states no part states: what it names is a whole model
            // per entry, and a face of it.
            Self::Build(_) => &[],
        }
    }
}

/// The part and state one `--state` argument names.
fn split_selection<'a>(spelling: &'a str, origin: &Origin) -> Result<(&'a str, &'a str), Fault> {
    spelling.split_once('=').ok_or_else(|| {
        Fault::about(
            origin.clone(),
            format!("`{spelling}` is not a state selection — one is written `part=state`"),
        )
        .in_field("state")
    })
}

/// Where a preview reads its materials from.
#[must_use]
pub fn materials_of(materials: Option<&PathBuf>) -> PathBuf {
    materials
        .cloned()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MATERIALS))
}

/// How many pixels a voxel spans, as asked for or by default.
#[must_use]
pub fn scale_of(requested: Option<u32>) -> u32 {
    requested.unwrap_or(DEFAULT_SCALE)
}

/// The command `argv` names.
///
/// # Errors
///
/// Returns a [`Fault`] carrying `clap`'s own message when the command line is
/// not one this tool understands. The origin is the tool rather than a
/// document, because at this point no document has been named.
pub fn parse(argv: Vec<OsString>) -> Result<Cli, Fault> {
    Cli::try_parse_from(argv)
        .map_err(|cause| Fault::about(Origin::new("voxforge"), cause.to_string()))
}
