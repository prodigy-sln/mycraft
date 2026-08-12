//! Reading back what a frame actually did.
//!
//! Both functions here map a buffer and wait for the device, which no frame may
//! do — they exist for the tests that need an *observation* rather than a
//! prediction. The frame statistics say how many sections the pure frustum
//! function admits; these say how many indices the compute pass compacted and
//! which sections it flagged. A renderer that admitted the right sections and
//! compacted the wrong quads is only distinguishable from here.
//!
//! Nothing on the render path calls either one, and nothing here is reachable
//! from a frame: the copy is recorded into an encoder of its own and submitted
//! on the spot.

use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::time::Duration;

use super::FrameError;
use super::buffers::SceneBuffers;

/// How long a readback may take before the device is called lost.
///
/// A liveness bound rather than a performance budget, matching the capture
/// harness's own: nothing here asserts how fast a buffer arrives, only that one
/// that never arrives cannot hang a test run.
const DEADLINE: Duration = Duration::from_secs(30);

/// Bytes in one `u32`.
const WORD_BYTES: u64 = 4;

/// The device a readback runs on, and the queue it submits its copy through.
///
/// A named pair rather than two loose arguments: every function here needs both
/// and neither is useful without the other.
#[derive(Debug, Clone, Copy)]
pub(super) struct Gpu<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

/// The indirect arguments' index count, as it stands after submission.
///
/// # Errors
///
/// Returns [`FrameError::Readback`] naming this stage when the device did not
/// hand the buffer over.
pub(super) fn drawn_index_count(buffers: &SceneBuffers, gpu: Gpu<'_>) -> Result<u32, FrameError> {
    let words = words_of(&buffers.args, WORD_BYTES, gpu, "indirect arguments")?;
    words.first().copied().ok_or(FrameError::Readback {
        stage: "indirect arguments",
    })
}

/// The visibility flag of each of `sections` sections, in section-index order.
///
/// # Errors
///
/// Returns [`FrameError::Readback`] naming this stage when the device did not
/// hand the buffer over.
pub(super) fn visible_sections(
    buffers: &SceneBuffers,
    gpu: Gpu<'_>,
    sections: u32,
) -> Result<Vec<u32>, FrameError> {
    if sections == 0 {
        return Ok(Vec::new());
    }
    words_of(
        &buffers.visible,
        WORD_BYTES * u64::from(sections),
        gpu,
        "visibility flags",
    )
}

/// The first `bytes` of `source`, as `u32`s.
///
/// The staging buffer belongs to this call and drops with it, so a readback that
/// failed leaves nothing behind for the next one to trip over.
fn words_of(
    source: &wgpu::Buffer,
    bytes: u64,
    gpu: Gpu<'_>,
    stage: &'static str,
) -> Result<Vec<u32>, FrameError> {
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mycraft terrain readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mycraft terrain readback"),
        });
    encoder.copy_buffer_to_buffer(source, 0, &staging, 0, bytes);
    gpu.queue.submit([encoder.finish()]);

    await_mapping(&staging, gpu.device, stage)?;
    let words = mapped_words(&staging, stage);
    staging.unmap();
    words
}

/// Waits for `staging` to become readable, bounded by the deadline.
fn await_mapping(
    staging: &wgpu::Buffer,
    device: &wgpu::Device,
    stage: &'static str,
) -> Result<(), FrameError> {
    let (sender, receiver) = mpsc::sync_channel(1);
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |outcome| {
            announce(&sender, &outcome);
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(DEADLINE),
        })
        .map_err(|_ignored| FrameError::Readback { stage })?;
    match receiver.recv_timeout(DEADLINE) {
        Ok(true) => Ok(()),
        Ok(false) | Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
            Err(FrameError::Readback { stage })
        }
    }
}

/// Reports the mapping's outcome to the waiting readback.
///
/// A send failure means the reader gave up on its deadline and is gone; there is
/// nobody left to tell. Deliberately not written `let _ = ...`, which is the
/// habit this shape exists not to teach.
fn announce(sender: &SyncSender<bool>, outcome: &Result<(), wgpu::BufferAsyncError>) {
    if sender.send(outcome.is_ok()).is_err() {
        // Nothing to do: the receiver went with the readback that timed out.
    }
}

/// The mapped bytes of `staging`, as `u32`s.
fn mapped_words(staging: &wgpu::Buffer, stage: &'static str) -> Result<Vec<u32>, FrameError> {
    let view = staging
        .slice(..)
        .get_mapped_range()
        .map_err(|_ignored| FrameError::Readback { stage })?;
    let words = view
        .chunks_exact(WORD_BYTES as usize)
        .map(|word| {
            let mut bytes = [0_u8; 4];
            bytes.copy_from_slice(word);
            u32::from_le_bytes(bytes)
        })
        .collect();
    drop(view);
    Ok(words)
}
