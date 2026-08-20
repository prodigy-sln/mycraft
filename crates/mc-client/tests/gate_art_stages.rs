//! The gate's two art stages, run for real: what they refuse, what they let
//! through, and what they put in front of whoever ran them.
//!
//! # Nothing here compares against a belief about the tool
//!
//! Both stages end in output that came from somewhere else — `git ls-files` for
//! one, `voxforge build` for the other — and a test that spelled out the expected
//! sentence would be comparing the gate against a third copy of somebody's idea
//! of it. So the refusal a stage has to reproduce is taken from a real run of the
//! tool, performed by the test, and the gate's output is asked whether it carries
//! that line. The one text written out by hand is the path of a fixture this file
//! committed, which is a fact about the repository rather than about the code
//! under test.
//!
//! # The fixture pair differs by one file
//!
//! `a-content-root` and `a-content-root-with-a-built-image` are the same three
//! files, and the second one also has a built image committed under `textures/`.
//! A stage that looked at a path that does not exist, or stopped reporting what it
//! found, would answer both the same way; making the pair a controlled comparison
//! is what turns that into two different verdicts.
//!
//! **They are content roots in the repository and not temporary trees**, because
//! the stage inspecting them runs `git ls-files` against the real repository and
//! git refuses a pathspec outside the worktree. `gate/mod.rs` records the
//! measurement.
//!
//! # The set is built into a copy, always
//!
//! `voxforge build` writes beside the manifest it is given, so every run here is
//! pointed at a copy in a temporary directory. Pointing it at the shipped manifest
//! would have four tests writing into `content/base/textures/` at once, and
//! pointing it at a tracked fixture would leave built images in the repository.

mod gate;

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use gate::reading::{GateScript, TestStageAfterARefusedBuild};
use gate::repository_root;
use gate::running::{
    ART_BUILD_STAGE, COMMITTED_SET_STAGE, GateReport, GateRun, SKIPPED_TEST_STAGE, a_copy_of,
    manifest_of,
};

type TestResult = Result<(), Box<dyn Error>>;

/// The content root carrying nothing but its own sources.
const A_CLEAN_ROOT: &str = "crates/mc-client/tests/fixtures/gate/a-content-root";

/// The same root, with one built image committed beside it.
const A_ROOT_WITH_A_BUILT_IMAGE: &str =
    "crates/mc-client/tests/fixtures/gate/a-content-root-with-a-built-image";

/// The one tracked path under that root's `textures/`, spelled as `git` reports
/// it. It is committed by this test's own fixture and is not derived from any run
/// of the stage that has to name it.
const THE_COMMITTED_IMAGE: &str = "crates/mc-client/tests/fixtures/gate/a-content-root-with-a-built-image/textures/fixture__block.png";

/// What one run of `voxforge build` said for itself.
#[derive(Debug)]
struct ToolRun {
    refused: bool,
    last_printed: String,
    last_complained: String,
}

/// Runs the set build exactly as the gate's stage has to, so the gate can be
/// asked whether it reproduced what the tool said.
fn what_the_tool_says(manifest: &Path) -> Result<ToolRun, Box<dyn Error>> {
    let finished = Command::new("cargo")
        .args(["run", "-p", "voxforge", "--quiet", "--", "build"])
        .arg(manifest)
        .current_dir(repository_root()?)
        .output()?;
    Ok(ToolRun {
        refused: !finished.status.success(),
        last_printed: last_line_of(&String::from_utf8(finished.stdout)?),
        last_complained: last_line_of(&String::from_utf8(finished.stderr)?),
    })
}

fn last_line_of(text: &str) -> String {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

/// A manifest no build can satisfy, in a directory of this test's own.
///
/// It refuses at the manifest, before any model is opened, so nothing is written
/// anywhere and the refusal is one line naming the file the caller handed over.
fn a_manifest_that_will_be_refused(directory: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let manifest = directory.join("textures.toml");
    fs::write(
        &manifest,
        "output           = \"textures\"\n\
         materials        = \"materials\"\n\
         blocks           = \"blocks\"\n\
         pixels_per_voxel = 0\n\n\
         [[texture]]\n\
         key   = \"fixture:block\"\n\
         model = \"models/block.mcvox\"\n\
         face  = \"top\"\n",
    )?;
    Ok(manifest)
}

#[test]
fn a_committed_built_image_fails_the_stage_naming_the_committed_path() -> TestResult {
    let building_into = a_copy_of("a-content-root")?;

    let run = GateRun::of_the_art_stages(A_ROOT_WITH_A_BUILT_IMAGE, &manifest_of(&building_into))?;

    assert_eq!(
        (
            run.report(),
            run.exit_code,
            run.writes_through(THE_COMMITTED_IMAGE)
        ),
        (
            GateReport::StagesFailed(vec![COMMITTED_SET_STAGE.to_owned()]),
            1,
            true
        ),
        "a built image under version control has to fail that stage alone and be named where \
         whoever ran the gate can read it. What the gate said:\n{}\n{}",
        run.printed,
        run.complained
    );
    Ok(())
}

#[test]
fn a_tree_carrying_only_the_manifest_models_and_materials_passes_the_stage() -> TestResult {
    let building_into = a_copy_of("a-content-root")?;

    let run = GateRun::of_the_art_stages(A_CLEAN_ROOT, &manifest_of(&building_into))?;

    assert_eq!(
        (run.report(), run.exit_code),
        (GateReport::EveryStageItRanPassed, 0),
        "a content root carrying its sources and no built image is the state this repository is \
         kept in, and both art stages have to pass on it. What the gate said:\n{}\n{}",
        run.printed,
        run.complained
    );
    Ok(())
}

/// What the tool itself says when it refuses `manifest`, which is the line the
/// gate has to put in front of whoever ran it.
///
/// Fails rather than returns when the run did not refuse or refused about
/// something else: an oracle nobody checked is a second thing that can be wrong.
fn the_refusal_of(manifest: &Path) -> Result<String, Box<dyn Error>> {
    let tool = what_the_tool_says(manifest)?;
    if !tool.refused || !tool.last_complained.contains("pixels per voxel") {
        return Err(format!(
            "the oracle for this test is a real refusal of the fixture manifest, and this run did \
             not produce one: refused {}, said {:?}",
            tool.refused, tool.last_complained
        )
        .into());
    }
    Ok(tool.last_complained)
}

#[test]
fn a_failing_set_build_fails_the_stage_and_reproduces_the_builds_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let manifest = a_manifest_that_will_be_refused(directory.path())?;
    let refusal = the_refusal_of(&manifest)?;

    let run = GateRun::of_the_art_stages(A_CLEAN_ROOT, &manifest)?;

    assert_eq!(
        (run.report(), run.exit_code, run.writes_through(&refusal)),
        (
            GateReport::StagesFailed(vec![
                ART_BUILD_STAGE.to_owned(),
                SKIPPED_TEST_STAGE.to_owned()
            ]),
            1,
            true
        ),
        "a refused build fails its stage, takes the test stage down with it, and puts the tool's \
         own words in front of whoever ran the gate. The tool said:\n{refusal}\nThe gate said:\n\
         {}\n{}",
        run.printed,
        run.complained
    );
    Ok(())
}

#[test]
fn a_failing_set_build_leaves_the_test_stage_unrun_and_records_that_it_did_not_run() -> TestResult {
    let directory = tempfile::tempdir()?;
    let manifest = a_manifest_that_will_be_refused(directory.path())?;

    let run = GateRun::of_the_art_stages(A_CLEAN_ROOT, &manifest)?;

    assert_eq!(
        (
            run.report(),
            GateScript::of_the_repository()?.test_stage_after_a_refused_build()
        ),
        (
            GateReport::StagesFailed(vec![
                ART_BUILD_STAGE.to_owned(),
                SKIPPED_TEST_STAGE.to_owned()
            ]),
            TestStageAfterARefusedBuild::TheTestsRunOnlyBesideTheRecordedSkip
        ),
        "the skip has to be recorded where a reader of the summary meets it, and the tests have to \
         be unreachable once the build refused — a summary listing one stage fewer cannot tell the \
         tests were not run from the tests not being in the list. What the gate said:\n{}\n{}",
        run.printed,
        run.complained
    );
    Ok(())
}

#[test]
fn an_art_build_naming_a_key_no_block_declares_still_passes_the_stage() -> TestResult {
    let building_into = a_copy_of("a-content-root")?;
    let manifest = manifest_of(&building_into);
    let tool = what_the_tool_says(&manifest)?;
    assert!(
        !tool.refused && tool.last_printed.contains("named by no block declaration"),
        "this test needs a build that completes while naming a key nothing declares, and this run \
         was not one: refused {}, said {:?}",
        tool.refused,
        tool.last_printed
    );

    let run = GateRun::of_the_art_stages(A_CLEAN_ROOT, &manifest)?;

    assert_eq!(
        (
            run.report(),
            run.exit_code,
            run.writes_through(&tool.last_printed)
        ),
        (GateReport::EveryStageItRanPassed, 0, true),
        "the unused-key report is advisory by design, and the shipped manifest names five of them \
         today. A stage keying on output rather than on the exit code goes red on the base game's \
         own art. What the gate said:\n{}\n{}",
        run.printed,
        run.complained
    );
    Ok(())
}
