//! A pointer that reports where it *is* rather than how far it moved, and the
//! turn the client makes of it.
//!
//! # The oracle is the pose the renderer is handed
//!
//! Every assertion below reads the `CameraPose` the simulation publishes, which
//! is the value the renderer is actually given — not a look delta, a yaw or any
//! other intermediate the product could stop consulting while still drawing the
//! same wrong picture. A camera's facing is `target - eye`, and "right" is taken
//! from the facing the run's own no-motion control published rather than written
//! down.
//!
//! # Every expected turn is a run of the untouched relative path
//!
//! The travel between two screen positions is a number of device counts, and a
//! client that has not been given a screen position at all spends device counts
//! as it always has. So each expectation here is a *second run of the same
//! client*, handed that travel as an ordinary delta — never a camera pose copied
//! out of a green run, and never a number this file works out from the pose it
//! is asserting about.
//!
//! # 135 counts, twice over, is not a coincidence
//!
//! The absolute range spans a declared 1920 × 1080, so one unit is `1920/65536`
//! counts horizontally and `1080/65536` vertically. 4608 units across and 8192
//! units down are therefore *each* exactly 135 counts, which is what makes the
//! two-axis scenario a comparison of one number against one number rather than a
//! ratio nobody can check by eye.
//!
//! # The recorded session is the one fixture nothing here could have written
//!
//! `fixtures/rdp_pointer/phase_b_cursor_grabbed.txt` is copied out of a probe run
//! by a human over a live Remote Desktop session. Nothing in this workspace can
//! produce a packet of that shape, so it is the only input in the suite that came
//! from the system that actually breaks rather than from someone reasoning about
//! it. Its expectation telescopes: differencing consecutive positions and adding
//! the differences leaves the first subtracted from the last, so the whole 312-
//! sample recording is worth exactly the travel between its first row and its
//! last — an oracle that never performs the differencing it is checking.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with
//! it the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired.

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;
use std::fs;
use std::path::Path;

use glam::Vec3;
use mc_sim::camera::CameraPose;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// A screen position to start from, well inside the absolute range.
const ORIGIN: (f64, f64) = (30_000.0, 20_000.0);

/// Absolute units of horizontal travel that come to [`TRAVEL_IN_COUNTS`].
const ACROSS: f64 = 4_608.0;

/// Absolute units of vertical travel that come to [`TRAVEL_IN_COUNTS`].
const DOWN: f64 = 8_192.0;

/// What either of those travels is worth in raw device counts.
const TRAVEL_IN_COUNTS: f64 = 135.0;

/// The width the absolute range is declared to span, in device counts.
const NOMINAL_WIDTH: f64 = 1920.0;

/// The height the absolute range is declared to span, in device counts.
const NOMINAL_HEIGHT: f64 = 1080.0;

/// How many steps the absolute range is divided into, per axis.
const ABSOLUTE_RANGE: f64 = 65_536.0;

/// The largest value the absolute range reports.
const ABSOLUTE_MAX: f64 = ABSOLUTE_RANGE - 1.0;

/// The smallest component that makes a sample look like a screen position.
///
/// The probe's own reading threshold: 584 of its 584 raw-motion samples clear
/// it, the smallest of them at 21 093.
const READING_THRESHOLD: f64 = 1000.0;

/// The recorded session, as this file expects to find it.
const RECORDING: &str = "tests/fixtures/rdp_pointer/phase_b_cursor_grabbed.txt";

/// How many samples the recording's grabbed phase delivered.
const RECORDED_SAMPLES: usize = 312;

/// How far apart two cameras may be looking and still have made the same turn.
///
/// Derived from both directions, and from a *measurement* of the arithmetic
/// rather than from reading the literals.
///
/// **Below it**: the recorded replay reaches the accumulator as 311 separate
/// `f32` additions where the control makes one, and those two totals really do
/// differ — by `3.6e-7` rad of yaw, computed offline from the fixture. At the
/// observable they differ by **exactly zero**, and the reason is the *magnitude
/// of the components*, not a distance. The eye sits at `(8.5, 11.62, 8.5)`
/// (`SPAWN` in `support/input/world.rs` plus `EYE_HEIGHT`) and the target is that
/// plus a unit direction, so **the largest component lies in `[8, 16)`, where
/// one `f32` step is `2^-20`, about `9.5e-7`** — larger than the disagreement,
/// which is therefore unrepresentable. So the floor is the pose's own
/// resolution, near `2e-6`, and not the accumulation.
///
/// The distance from the origin is 16.7 and has nothing to do with it; an earlier
/// version of this comment said `8.5 units from the origin`, which was one
/// component mistaken for a radius — **the right number for the wrong quantity,
/// which is the exact failure this constant's own history is about**
/// (`docs/technical/testing.md`, beside check 7).
///
/// What the corrected reason buys is a threshold worth watching: the step doubles
/// the moment any component reaches 16, and the largest component is not the eye's
/// `11.62` — the comparison is built from the eye *and* the target, and the target
/// is the eye plus a unit direction, so its `y` reaches `12.62`. This fixture has
/// **3.38 blocks of headroom**, and **four** blocks of extra spawn height is the
/// smallest whole number that crosses it: `12.62 + 3 = 15.62` stays under, and
/// `12.62 + 4 = 16.62` does not. Raise `SPAWN` that far and the floor becomes
/// `1.9e-6`.
///
/// **Above it**: the smallest difference this comparison must still catch is one
/// dropped sample of the recording, whose mean step is 419 units — 12.3 counts,
/// or 0.027 rad.
///
/// `1e-3` sits some 500× above the first and 27× below the second. It is **not**
/// tightened to bit equality even though this fixture would pass it: an
/// over-tight assertion fails against a correct camera the day the fixture spawns
/// high enough to coarsen the step, and the cheapest way to green that is to
/// round something in the product.
const SAME_TURN: f32 = 1e-3;

#[test]
fn the_first_screen_position_of_a_session_turns_the_camera_by_nothing() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let moved = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let one_position = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(ORIGIN.0, ORIGIN.1);
    })?;

    assert!(
        rightward_lean(&moved, &still) > 0.0,
        "the control this scenario needs: the same client, the same world and the same tick, \
         handed {TRAVEL_IN_COUNTS} counts of ordinary motion, has to turn the camera — or the \
         sameness below is a client whose pointer reaches nothing at all. It leaned {}",
        rightward_lean(&moved, &still)
    );
    assert_eq!(
        one_position, still,
        "a screen position on its own says where the pointer is, not how far it moved, and there \
         is nothing yet to measure it from. So the first one of a session leaves the published \
         camera exactly as a tick with no motion at all leaves it. This is the packet that spins \
         the camera 7.8 revolutions today: {} × the look sensitivity is 49 radians from a single \
         event, and a client that spends it has already thrown the player's view away before it \
         could know what kind of stream it was reading",
        ORIGIN.0
    );
    Ok(())
}

#[test]
fn a_second_screen_position_turns_the_camera_by_the_travel_between_them() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let travelled = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let positions = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(ORIGIN.0, ORIGIN.1);
        harness.move_pointer(ORIGIN.0 + ACROSS, ORIGIN.1);
    })?;

    assert!(
        rightward_lean(&travelled, &still) > 0.0,
        "the control this scenario is read against: {TRAVEL_IN_COUNTS} counts of ordinary motion \
         turn the camera right. It leaned {}",
        rightward_lean(&travelled, &still)
    );
    assert_eq!(
        positions, travelled,
        "two screen positions {ACROSS} units apart are the pointer having travelled \
         {TRAVEL_IN_COUNTS} counts, and the camera turns by that and by nothing else — landing \
         exactly where the same client lands when it is handed those counts as an ordinary \
         delta. The failure this excludes is the one on the ticket: spending either position \
         whole is a turn of thirty thousand counts, two hundred times a full revolution, from a \
         pointer that moved a seventh of the screen"
    );
    Ok(())
}

#[test]
fn each_further_screen_position_turns_the_camera_by_the_travel_since_the_last() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let once = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let twice = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
        harness.move_pointer(TRAVEL_IN_COUNTS, 0.0);
    })?;
    let three_positions = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(ORIGIN.0, ORIGIN.1);
        harness.move_pointer(ORIGIN.0 + ACROSS, ORIGIN.1);
        harness.move_pointer(ORIGIN.0 + ACROSS + ACROSS, ORIGIN.1);
    })?;

    assert!(
        rightward_lean(&twice, &still) > rightward_lean(&once, &still),
        "the control this scenario is read against: twice the ordinary motion has to lean \
         further right than once. It leaned {} against {}",
        rightward_lean(&twice, &still),
        rightward_lean(&once, &still)
    );
    assert_eq!(
        three_positions, twice,
        "each position after the first is measured from the position before it, never from where \
         the session opened: three positions {ACROSS} units apart in turn are two travels of \
         {TRAVEL_IN_COUNTS} counts. A client that kept differencing against the first sample \
         would turn by {TRAVEL_IN_COUNTS} and then by twice it, which is a camera that \
         accelerates as long as the player keeps moving the pointer one way"
    );
    Ok(())
}

#[test]
fn the_recorded_remote_desktop_session_turns_the_camera_by_the_travel_it_recorded() -> TestResult {
    let recorded = the_recording_as_delivered()?;
    let net = net_counts(&recorded)?;
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let replayed = camera_after_one_tick(InputHarness::started(), |harness| {
        for sample in &recorded {
            harness.move_pointer(sample.0, sample.1);
        }
    })?;
    let travelled = camera_after_one_tick(InputHarness::started(), |harness| {
        harness.move_pointer(net.0, net.1);
    })?;

    assert!(
        looking_apart(&travelled, &still) > SAME_TURN,
        "the control this scenario needs: the recording's net travel of {net:?} counts has to \
         turn the camera measurably, or the agreement below is two runs that both looked \
         nowhere. Its facing sits {} away from the still one",
        looking_apart(&travelled, &still)
    );
    assert!(
        looking_apart(&replayed, &travelled) < SAME_TURN,
        "the {RECORDED_SAMPLES} raw samples a real Remote Desktop session delivered while it held \
         the cursor are worth the distance the pointer travelled across them and nothing more — \
         the same turn the untouched relative path makes of {net:?} counts. The two facings sit \
         {} apart. Spending the raw values instead is four orders of magnitude more: some 33 000 \
         radians of yaw, and a pitch driven hard into its own clamp, which is a camera staring \
         at the floor after the player nudged the mouse",
        looking_apart(&replayed, &travelled)
    );
    Ok(())
}

#[test]
fn the_same_travel_in_either_axis_turns_the_camera_by_the_same_angle() -> TestResult {
    let still = camera_after_one_tick(InputHarness::started(), |_| {})?;
    let across = camera_after_one_tick(InputHarness::started(), |harness| {
        engaged(harness);
        harness.move_pointer(ORIGIN.0 + ACROSS, ORIGIN.1);
    })?;
    let down = camera_after_one_tick(InputHarness::started(), |harness| {
        engaged(harness);
        harness.move_pointer(ORIGIN.0, ORIGIN.1 + DOWN);
    })?;

    let turned_across = (yaw(&across) - yaw(&still)).abs();
    let turned_down = (pitch(&still) - pitch(&down)).abs();

    assert!(
        turned_across > SAME_TURN,
        "the control this scenario needs: {ACROSS} units across has to turn the camera at all, or \
         the agreement below is two axes that both do nothing. It turned {turned_across} radians"
    );
    assert!(
        (turned_across - turned_down).abs() < SAME_TURN,
        "the absolute range is normalised per axis over the display, so {ACROSS} units across and \
         {DOWN} units down are the same distance travelled — {TRAVEL_IN_COUNTS} counts each — and \
         have to turn the camera by the same angle. They turned {turned_across} and \
         {turned_down}. Differencing the raw stream and handing both axes to one sensitivity is \
         the shape that fails here: it leaves the vertical axis 1.83 times as fast as the \
         horizontal, which is smooth, total and reproducible while being unplayable"
    );
    Ok(())
}

/// Two screen positions at `ORIGIN`, which engages the absolute reading without
/// turning the camera: the pointer was still between them, so there is no travel
/// to spend.
fn engaged(harness: &mut InputHarness) {
    harness.move_pointer(ORIGIN.0, ORIGIN.1);
    harness.move_pointer(ORIGIN.0, ORIGIN.1);
}

/// Whether `sample` is shaped like a screen position: both components inside the
/// absolute range, at least one of them above the recording's reading threshold.
fn is_a_screen_position(sample: &(f64, f64)) -> bool {
    let (x, y) = *sample;
    let in_range = (0.0..=ABSOLUTE_MAX).contains(&x) && (0.0..=ABSOLUTE_MAX).contains(&y);
    in_range && (x >= READING_THRESHOLD || y >= READING_THRESHOLD)
}

/// What the whole recording is worth, in device counts.
///
/// The differences telescope, so this never differences anything: it is the last
/// row of the recording minus the first, and nothing in between is read.
fn net_counts(recorded: &[(f64, f64)]) -> Result<(f64, f64), Box<dyn Error>> {
    let first = *recorded.first().ok_or("the recording has a first sample")?;
    let last = *recorded.last().ok_or("the recording has a last sample")?;
    Ok(counts_between(first, last))
}

/// The device counts between two screen positions, per axis.
fn counts_between(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    (
        (to.0 - from.0) * NOMINAL_WIDTH / ABSOLUTE_RANGE,
        (to.1 - from.1) * NOMINAL_HEIGHT / ABSOLUTE_RANGE,
    )
}

/// The recorded session, checked against the two things the expectation above
/// rests on and no assertion in the test itself could enforce.
///
/// The count catches a truncated fixture or a parser that has stopped
/// recognising the probe's rows. Every sample being a screen position is what
/// makes the differences telescope down to the last row minus the first — one
/// ordinary delta anywhere in the recording would break the sum this test never
/// performs.
fn the_recording_as_delivered() -> Result<Vec<(f64, f64)>, Box<dyn Error>> {
    let recorded = recorded_samples()?;
    assert_eq!(
        recorded.len(),
        RECORDED_SAMPLES,
        "the fixture this scenario is stated over: the grabbed phase of the recording delivered \
         {RECORDED_SAMPLES} raw-motion samples"
    );
    assert!(
        recorded.iter().all(is_a_screen_position),
        "the fixture's other precondition: every recorded sample has to be a screen position, or \
         the travel between the first row and the last is not what the replay is worth"
    );
    Ok(recorded)
}

/// The raw samples of the recorded session, in the order it delivered them.
///
/// A data row is the probe's own five columns; comments and blanks are the
/// provenance the fixture carries and are skipped. A row this cannot read is an
/// error rather than a silently dropped sample, because a parser that quietly
/// skipped half the recording would leave a shorter replay agreeing with a
/// shorter expectation.
fn recorded_samples() -> Result<Vec<(f64, f64)>, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(RECORDING);
    let text = fs::read_to_string(&path)?;
    let mut samples = Vec::new();
    for line in text.lines() {
        let row = line.trim();
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let mut columns = row.split_whitespace().skip(3);
        let x = columns.next().ok_or("a data row states an x")?;
        let y = columns.next().ok_or("a data row states a y")?;
        samples.push((x.parse::<f64>()?, y.parse::<f64>()?));
    }
    Ok(samples)
}

/// The camera one tick publishes, with `dispatched` delivered to a client over
/// the declared ground plane before that tick is taken.
fn camera_after_one_tick(
    mut harness: InputHarness,
    dispatched: impl FnOnce(&mut InputHarness),
) -> Result<CameraPose, Box<dyn Error>> {
    harness.start_world()?;
    dispatched(&mut harness);
    let published = harness
        .tick()
        .ok_or("a tick over a started world publishes a snapshot")?;
    Ok(published.camera)
}

/// The direction a published camera looks in.
fn facing(camera: &CameraPose) -> Vec3 {
    Vec3::from_array(camera.target) - Vec3::from_array(camera.eye)
}

/// How far `turned` leans toward the right hand of what `control` was facing.
fn rightward_lean(turned: &CameraPose, control: &CameraPose) -> f32 {
    let ahead = facing(control);
    facing(turned).dot(Vec3::new(-ahead.z, 0.0, ahead.x))
}

/// Where a published camera is pointed in the horizontal plane, in radians.
fn yaw(camera: &CameraPose) -> f32 {
    let ahead = facing(camera);
    ahead.z.atan2(ahead.x)
}

/// How far above level a published camera is pointed, in radians.
fn pitch(camera: &CameraPose) -> f32 {
    facing(camera).normalize_or_zero().y.asin()
}

/// How far apart two published cameras are looking, as the straight-line
/// distance between their two unit facings.
///
/// **A chord, and deliberately not `Vec3::angle_between`.** That call is
/// `acos(dot)`, and for two nearly parallel `f32` unit vectors the dot product
/// is `1 - d^2/2`: at `d = 3e-5` that is `1 - 4.5e-10`, which rounds to exactly
/// `1.0` and comes back as a flat zero. Measured, on this file's own recorded
/// replay: it reported `0`. So an angle taken that way cannot resolve anything
/// below about `5e-4`, and a tolerance near that number would be sitting on the
/// instrument's own noise floor rather than above the arithmetic's. A chord is
/// linear in the angle, equals it to within `theta^2/24`, and stays readable
/// down to the precision of the components themselves.
fn looking_apart(one: &CameraPose, other: &CameraPose) -> f32 {
    (facing(one).normalize_or_zero() - facing(other).normalize_or_zero()).length()
}
