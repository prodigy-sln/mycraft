//! What a frame reports about the snapshot it was handed.
//!
//! Reading a stale snapshot is correct; stalling the simulation is not. The
//! renderer therefore keeps no record of which tick it last drew — with nothing
//! stored there is no comparison that could refuse an older snapshot or wait for
//! a newer one — and it reports the tick it was given.
//!
//! That absence is what this test is about, and it is why the statistics are
//! computed by a free function over a snapshot and a projection rather than by a
//! method on something that could remember. An implementation that wanted to
//! refuse a stale snapshot would need somewhere to keep the last tick, and there
//! is nowhere: the sequence below hands over tick 60 and then tick 12, and the
//! second call cannot know the first happened.

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::{Projection, camera_view};
use mc_render::geometry::scene::SceneGeometry;
use mc_render::geometry::{SectionOrigin, build_section_geometry};
use mc_render::snapshot::{TerrainSnapshot, frame_stats};
use mc_render::texture::TextureLayers;
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

type TestResult = Result<(), Box<dyn Error>>;

/// The block the one-quad scene below is made of. An `example:` namespace: a
/// fixture borrowing a shipped block name would be the engine describing itself
/// in terms of content.
const PROBE: &str = "example:probe";

/// The tick just rendered, and the older one handed over next.
const NEWER_TICK: u32 = 60;
const OLDER_TICK: u32 = 12;

/// How many sections the fixture scene holds.
const SECTIONS: u32 = 1;

#[test]
fn a_snapshot_older_than_the_one_just_rendered_is_rendered_and_reports_its_own_tick() -> TestResult
{
    let scene = Arc::new(one_section_scene()?);
    let projection = declared_projection();

    let newer = frame_stats(&snapshot(NEWER_TICK, &scene), &projection);
    let older = frame_stats(&snapshot(OLDER_TICK, &scene), &projection);

    assert_eq!(
        newer.tick, NEWER_TICK,
        "the newer snapshot has to be rendered first, or the older one below is not older \
         than anything"
    );
    assert_eq!(
        (
            older.tick,
            older.sections_submitted,
            older.terrain_draw_calls
        ),
        (OLDER_TICK, SECTIONS, 1),
        "the older snapshot is drawn like any other and reports its own tick, rather than \
         being refused or held back for a newer one"
    );
    Ok(())
}

/// A snapshot of `scene` at `tick`, from a camera that is not what this test is
/// about.
fn snapshot(tick: u32, scene: &Arc<SceneGeometry>) -> TerrainSnapshot {
    TerrainSnapshot {
        tick,
        camera: camera_view([0.0, 0.0, 0.0], [0.0, 0.0, -64.0]),
        scene: Arc::clone(scene),
    }
}

/// The projection the replay declares.
fn declared_projection() -> Projection {
    Projection {
        fov_y_radians: 60.0_f32.to_radians(),
        aspect: 1280.0 / 720.0,
        near: 0.5,
        far: 512.0,
    }
}

/// A scene of one section holding one upward-facing quad.
fn one_section_scene() -> Result<SceneGeometry, Box<dyn Error>> {
    let layers = TextureLayers::resolve(&BTreeSet::from([TextureKey::parse(PROBE)?]));
    let quad = Quad {
        facing: Facing::PosY,
        plane: 0,
        origin: PlanePos {
            primary: 0,
            secondary: 0,
        },
        extent: PlaneExtent {
            primary: 1,
            secondary: 1,
        },
        block: BlockName::parse(PROBE)?,
    };
    let section = build_section_geometry(&[quad], SectionOrigin::new([0, 0, 0]), &layers)?;
    Ok(SceneGeometry::assemble(vec![section])?)
}
