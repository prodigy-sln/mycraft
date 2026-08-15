//! What an inspect report looks like as text.
//!
//! One fact per line, because the reader is an agent: a line is a thing you can
//! grep for, and a paragraph is not. The filled count sits behind `filled ` as
//! the next whitespace-separated token, so the wording around it can change
//! without breaking anybody parsing it.

use std::io::Write;

use crate::inspect::{Bounds, Floating, Observation, Report};

/// Writes `report` to `out`, one fact per line.
///
/// # Errors
///
/// Returns whatever `out` failed with.
pub fn write(report: &Report, out: &mut dyn Write) -> std::io::Result<()> {
    let stats = report.stats();
    writeln!(out, "filled {} voxels", stats.filled)?;
    write_bounds(stats.bounds, out)?;
    for count in &stats.materials {
        writeln!(out, "  {} {}", count.material.as_str(), count.voxels)?;
    }
    for observation in report.observations() {
        write_observation(observation, out)?;
    }
    for defect in report.defects() {
        write_defect(defect, out)?;
    }
    Ok(())
}

/// Writes the bounding box, or the fact that there is none.
fn write_bounds(bounds: Bounds, out: &mut dyn Write) -> std::io::Result<()> {
    match bounds {
        Bounds::Empty => writeln!(out, "bounds none"),
        Bounds::Spanning { lowest, highest } => writeln!(
            out,
            "bounds ({}, {}, {}) to ({}, {}, {}) inclusive",
            lowest.x, lowest.y, lowest.z, highest.x, highest.y, highest.z
        ),
    }
}

/// Writes one observation.
fn write_observation(observation: &Observation, out: &mut dyn Write) -> std::io::Result<()> {
    match observation {
        Observation::Connectivity {
            components,
            floating,
        } => {
            writeln!(out, "components {}", components.len())?;
            write_floating(floating, out)
        }
        Observation::Symmetry { axis, verdict } => {
            writeln!(out, "symmetry {} {verdict:?}", axis.as_str())
        }
    }
}

/// Writes which voxels touch nothing, naming every one of them.
///
/// Every one, not the first: a pair of parts each attached to nothing is two
/// things to repair, and naming one reads as one.
fn write_floating(floating: &Floating, out: &mut dyn Write) -> std::io::Result<()> {
    let Floating::Detached(voxels) = floating else {
        return writeln!(out, "floating none");
    };
    writeln!(out, "floating {}", voxels.len())?;
    for at in voxels {
        writeln!(out, "  ({}, {}, {})", at.x, at.y, at.z)?;
    }
    Ok(())
}

/// Writes one defect.
fn write_defect(defect: &crate::inspect::Defect, out: &mut dyn Write) -> std::io::Result<()> {
    let crate::inspect::Defect::UnusedPaletteEntry { key } = defect;
    let spelling = char::from(*key);
    writeln!(
        out,
        "defect: the palette declares `{spelling}` and no grid spells it"
    )
}
