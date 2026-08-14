//! A content root that names the debug overlay gets an element, not the overlay.
//!
//! The overlay is the instrument somebody diagnoses a misbehaving mod *with*, so
//! a mod that could reach it could disable the one thing that would have reported
//! it. The guarantee is held by there being nothing to reach: a declaration states
//! a name, an anchor, a rectangle and a colour, and no field of it refers to the
//! overlay however the name is spelled. `base:debug-overlay` is therefore an
//! ordinary element with an unusual name — it composes a rectangle at the anchor
//! it names, exactly as `base:crosshair-horizontal` does, and the overlay carries
//! on doing whatever it was doing.
//!
//! # Read through the client's own startup, and that is why this file is here
//!
//! What "registers it" means is what `prepare_scene` registered, because that is
//! the reading a launched client does. This scenario needs *both* the loader and
//! the overlay's own state, and the client is the only crate that resolves both:
//! `mc-render` owns the overlay and depends on `mc-world`, so the dependency
//! direction leaves nowhere else for a test that needs the two together.
//!
//! # What each half of the assertion can and cannot see, stated plainly
//!
//! **The registration half is falsifiable and is the weight of this scenario.** A
//! loader that refused the name, or that quietly dropped it, or that appended it
//! after the elements a content root declares in sorted order, fails here — and
//! the position it must land at is derived from the root's own file names rather
//! than written down, so a fourth shipped declaration would not move it.
//!
//! **The visibility half is a claim about there being no path at all**, and it is
//! worth being exact about how much that is worth: nothing in the reading below
//! is handed an overlay, so a special case would have to be *added* before this
//! could fail. What it does close is the reading having *acquired* one — a loader
//! that grew an opinion about a name it recognises. Both directions are asserted,
//! a shown overlay and a hidden one, so "unchanged" means unchanged rather than
//! "still the state everything starts in". The stronger form of this claim is a
//! frame with an opaque element over the whole target and the overlay still
//! legible on top of it, and that belongs to the phase that paints one.

mod support;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use mc_core::hud::{Anchor, Draw, HudElement, HudLayout, Rgba8};
use mc_render::overlay::DebugOverlay;

use support::{TestResult, content};

/// The file the fixture declaration is written into, and the name it declares.
///
/// The name is the one a mod would reach for if reaching were possible, and the
/// file name is deliberately not first or last in sorted order among the root's
/// declarations, so an element appended rather than sorted lands somewhere this
/// test can see.
const FIXTURE_FILE: &str = "debug-overlay.toml";
const OVERLAY_NAME: &str = "base:debug-overlay";

/// The declaration, stating every field an ordinary element states and nothing
/// else — there is no other kind of field to state.
const DECLARATION: &str = "name = \"base:debug-overlay\"\nanchor = \"top-left\"\nsize = [4, 4]\n\
     draw = \"fill\"\ncolor = \"#0F1E2DFF\"\noutline = \"#0A0B0CFF\"\n";

/// What that declaration says, in the model's own vocabulary.
///
/// Written out from the declaration above rather than read back from what the
/// loader made of it: an expectation taken from the subject agrees with the
/// subject whatever the subject did.
const DECLARED_SHAPE: Shape = Shape {
    anchor: Anchor::TopLeft,
    offset: [0, 0],
    size: [4, 4],
    draw: Draw::Fill {
        color: Rgba8 {
            r: 0x0F,
            g: 0x1E,
            b: 0x2D,
            a: 0xFF,
        },
    },
    outline: Some(Rgba8 {
        r: 0x0A,
        g: 0x0B,
        b: 0x0C,
        a: 0xFF,
    }),
};

/// Everything an element is, apart from the name it is looked up by and the file
/// it came from.
///
/// A value of its own so the comparison is one `assert_eq!` a reader can check
/// against the declaration, rather than five.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shape {
    anchor: Anchor,
    offset: [i32; 2],
    size: [u32; 2],
    draw: Draw,
    outline: Option<Rgba8>,
}

impl Shape {
    /// What `element` is, apart from its name and its origin.
    fn of(element: &HudElement) -> Self {
        Self {
            anchor: element.anchor,
            offset: element.offset,
            size: element.size,
            draw: element.draw,
            outline: element.outline,
        }
    }
}

/// What reading a content root that names the overlay left behind: where the
/// element landed, what it turned out to be, and what the two overlays are doing
/// afterwards.
#[derive(Debug, PartialEq, Eq)]
struct Reading {
    at: Option<usize>,
    shape: Option<Shape>,
    still_shown: bool,
    still_hidden: bool,
}

impl Reading {
    /// What `registered` holds under the fixture's name, and what `shown` and
    /// `hidden` are doing now.
    fn of(registered: &HudLayout, shown: &DebugOverlay, hidden: &DebugOverlay) -> Self {
        let at = registered
            .elements()
            .iter()
            .position(|element| element.name.as_str() == OVERLAY_NAME);
        Self {
            at,
            shape: at
                .and_then(|at| registered.elements().get(at))
                .map(Shape::of),
            still_shown: shown.visible(),
            still_hidden: hidden.visible(),
        }
    }
}

/// Where a declaration file named `file_name` sorts among the `*.toml` files
/// under `root`'s `hud/` directory.
///
/// The oracle for "registered like any other element": the order elements are
/// registered in is the order their files sort in, so an element the loader
/// treated as a special case lands somewhere else. Derived from a directory
/// listing and `sort`, which share no code with the loader.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
fn sorted_position_of(root: &Path, file_name: &str) -> Result<Option<usize>, Box<dyn Error>> {
    let mut declared = Vec::new();
    for entry in fs::read_dir(root.join(content::HUD_DIRECTORY))? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new("toml"))
            && let Some(name) = path.file_name().and_then(OsStr::to_str)
        {
            declared.push(name.to_owned());
        }
    }
    declared.sort();
    Ok(declared.iter().position(|declared| declared == file_name))
}

/// Refuses to go on unless one of the two overlays is shown and the other is not.
///
/// The control the visibility half is stated under: two hidden overlays make
/// "unchanged" mean nothing more than "still the state everything starts in", and
/// the claim would then hold against a reading that hid one.
///
/// # Errors
///
/// Returns an error naming what each of the two is doing when they are not one of
/// each.
fn require_one_shown_and_one_hidden(
    shown: &DebugOverlay,
    hidden: &DebugOverlay,
) -> Result<(), Box<dyn Error>> {
    if shown.visible() && !hidden.visible() {
        return Ok(());
    }
    Err(format!(
        "this scenario has to start with one overlay shown and one hidden, so that 'the \
         visibility is unchanged' is a claim about both states rather than about the default one. \
         It started with {shown} and {hidden}",
        shown = shown.visible(),
        hidden = hidden.visible()
    )
    .into())
}

#[test]
fn an_element_named_after_the_debug_overlay_registers_ordinarily_and_leaves_the_overlay_alone()
-> TestResult {
    let root = content::shipped_with(FIXTURE_FILE, DECLARATION)?;
    let mut shown = DebugOverlay::default();
    shown.toggle();
    let hidden = DebugOverlay::default();
    require_one_shown_and_one_hidden(&shown, &hidden)?;

    let registered = mc_client::startup::prepare_scene(root.path())?.hud;

    assert_eq!(
        Reading::of(&registered, &shown, &hidden),
        Reading {
            at: sorted_position_of(root.path(), FIXTURE_FILE)?,
            shape: Some(DECLARED_SHAPE),
            still_shown: true,
            still_hidden: false,
        },
        "`{OVERLAY_NAME}` is a name, and a name is all it is: the element registers where its file \
         sorts, carrying the rectangle and the colours it declared, exactly as every other \
         declaration in the root does. And the overlay it is named after carries on doing whatever \
         it was doing — a mod that could turn it off could turn off the instrument somebody would \
         have diagnosed that mod with, which is why no field of a declaration refers to it and why \
         a recognised name earns no special treatment. The root registered: {:?}",
        registered
            .elements()
            .iter()
            .map(|element| element.name.as_str())
            .collect::<Vec<_>>()
    );
    Ok(())
}
