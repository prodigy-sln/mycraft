//! The offscreen colour target: allocate it, let the caller draw into it, copy
//! it out, and hand back tightly packed pixels.
//!
//! Two properties of this file are load-bearing and neither is visible in its
//! shape.
//!
//! **No stage flips rows.** Texture row 0 is the top of the render target and
//! stays the top through the copy, the unpadding and everything downstream. A
//! capture path that inverted rows would make every golden this project ever
//! commits wrong in the same direction — consistently, and therefore invisibly.
//!
//! **Nothing is allocated per context.** The texture and the readback buffer
//! belong to one capture and drop at the end of it, so a capture that failed
//! leaves nothing behind for the next one to trip over.

use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::time::Duration;

use crate::frame::clock::{CaptureError, Progress, SystemClock, poll_until_deadline};
use crate::frame::image::Rgba8Image;
use crate::frame::readback::{BYTES_PER_PIXEL, ReadbackError, padded_row_bytes, unpad_rows};

use super::{Capture, CaptureRequest, DrawWork};

/// How long to wait between two non-blocking polls of the device.
///
/// [`poll_until_deadline`] never sleeps — that is what keeps the fake-clock test
/// instantaneous — so the pacing lives here, beside the poll it paces. Without
/// it the wait would spin a core for the whole readback.
const POLL_INTERVAL: Duration = Duration::from_micros(100);

/// The one capture format: 8-bit RGBA with the sRGB encode performed by the
/// hardware, which is the path a renderer will use and the one thing that must
/// not be re-implemented on the CPU.
///
/// Public because a renderer has to configure its colour target to match, and a
/// second literal on that side is a second place the two can drift apart — the
/// captured frame would then be read back through a format the pass never wrote.
/// Exported rather than merely readable: this is the value a consumer asserts
/// its own against.
pub const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Captures one frame.
///
/// The size arrives as a [`crate::frame::FrameSize`], which only
/// `validate_frame_size` can produce, so a frame no device could render is
/// rejected before this function is ever called — that is what makes "SHALL NOT
/// submit any GPU work" structural rather than a promise.
pub(super) fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    request: &CaptureRequest,
    draw: &mut dyn DrawWork,
) -> Result<Capture, CaptureError> {
    let target = Target::allocate(device, request).map_err(CaptureError::Readback)?;
    target.draw(device, queue, draw)?;
    let readback = target.wait(device, request)?;
    let pixels = target.pixels()?;
    let image =
        Rgba8Image::from_rgba(target.width, target.height, pixels).map_err(CaptureError::Shape)?;
    Ok(Capture { image, readback })
}

/// One capture's texture and the buffer its pixels are copied into.
struct Target {
    texture: wgpu::Texture,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    /// Bytes of image data in a row, without the copy alignment's filler.
    row_bytes: u32,
    /// Bytes of buffer a row occupies, filler included.
    stride: u32,
}

impl Target {
    fn allocate(device: &wgpu::Device, request: &CaptureRequest) -> Result<Self, ReadbackError> {
        let (width, height) = (request.size.width(), request.size.height());
        let row_bytes = width
            .checked_mul(BYTES_PER_PIXEL)
            .ok_or(ReadbackError::RowTooWide { width })?;
        let stride = padded_row_bytes(width)?;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mycraft capture target"),
            size: extent(width, height),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: CAPTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mycraft capture readback"),
            // Two `u32`s widened to `u64` cannot overflow their product.
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Self {
            texture,
            buffer,
            width,
            height,
            row_bytes,
            stride,
        })
    }

    /// Records the caller's draw work and the copy that follows it, then submits.
    ///
    /// The caller's failure returns before the submission, so a scene that could
    /// not be recorded costs the device nothing.
    fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draw: &mut dyn DrawWork,
    ) -> Result<(), CaptureError> {
        let view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mycraft capture"),
        });

        draw.record(&mut encoder, &view)
            .map_err(CaptureError::DrawWork)?;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(self.height),
                },
            },
            extent(self.width, self.height),
        );
        queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Waits for the copy to land in mappable memory, bounded by the deadline.
    fn wait(
        &self,
        device: &wgpu::Device,
        request: &CaptureRequest,
    ) -> Result<Duration, CaptureError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |outcome| {
                announce(&sender, outcome)
            });

        let clock = SystemClock::started_now();
        poll_until_deadline(&clock, request.deadline, || poll_once(device, &receiver))
            .map(|elapsed| elapsed.elapsed)
            .map_err(|expired| expired.into_capture_error(request.capture.clone()))
    }

    /// The mapped bytes with the copy alignment's filler stripped out.
    fn pixels(&self) -> Result<Vec<u8>, CaptureError> {
        let slice = self.buffer.slice(..);
        let view = slice.get_mapped_range().map_err(|cause| {
            CaptureError::Readback(ReadbackError::DeviceLost {
                cause: cause.to_string(),
            })
        })?;
        let pixels = unpad_rows(
            &view,
            self.row_bytes as usize,
            self.stride as usize,
            self.height,
        )
        .map_err(CaptureError::Readback)?;

        // The view borrows the mapping, so it has to go before the unmap.
        drop(view);
        self.buffer.unmap();
        Ok(pixels)
    }
}

/// Reports the mapping's outcome to the waiting capture.
///
/// A send failure means the capture gave up on its deadline and is gone; there
/// is nobody left to tell, and deliberately not written `let _ = ...` — a
/// discarded `must_use` is the habit this shape exists not to teach.
fn announce(sender: &SyncSender<bool>, outcome: Result<(), wgpu::BufferAsyncError>) {
    if sender.send(outcome.is_ok()).is_err() {
        // Nothing to do: the receiver went with the capture that timed out.
    }
}

/// One non-blocking look at the device, and at what the mapping has reported.
fn poll_once(
    device: &wgpu::Device,
    receiver: &Receiver<bool>,
) -> Result<Progress<()>, ReadbackError> {
    device
        .poll(wgpu::PollType::Poll)
        .map_err(|cause| ReadbackError::DeviceLost {
            cause: cause.to_string(),
        })?;

    match receiver.try_recv() {
        Ok(true) => Ok(Progress::Ready(())),
        Ok(false) => Err(ReadbackError::DeviceLost {
            cause: "the readback buffer could not be mapped for reading".to_owned(),
        }),
        Err(TryRecvError::Empty) => {
            std::thread::sleep(POLL_INTERVAL);
            Ok(Progress::Pending)
        }
        Err(TryRecvError::Disconnected) => Err(ReadbackError::DeviceLost {
            cause: "the device dropped the readback without reporting it".to_owned(),
        }),
    }
}

/// A flat 2D extent: one colour target, no depth, no array layers.
const fn extent(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}
