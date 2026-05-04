//! GPU → CPU readback for RGBA8/BGRA8 textures. Handles the 256-byte row
//! alignment that wgpu requires for buffer copies and strips it on the way out.

use crate::gpu::device::{GpuContext, GpuError};

const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Padded bytes-per-row for a `width`-pixel × 4-bytes-per-pixel texture,
/// rounded up to wgpu's 256-byte row alignment requirement.
fn padded_row_bytes(width: u32) -> u32 {
    let unpadded = width * 4;
    let padding = (ROW_ALIGN - unpadded % ROW_ALIGN) % ROW_ALIGN;
    unpadded + padding
}

/// Allocate a readback buffer sized for an 8 bits-per-channel `width × height`
/// texture (`COPY_DST | MAP_READ`).  Sized to the padded row stride so it can
/// be reused across calls — buffers can be unmapped and re-mapped.  Split from
/// the read so the texture cache can hold the buffer alongside the textures.
pub fn make_readback_buffer(ctx: &GpuContext, width: u32, height: u32) -> wgpu::Buffer {
    let buffer_size = u64::from(padded_row_bytes(width)) * u64::from(height);
    ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rpview-gpu readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

/// Copy an `Rgba8Unorm`/`Bgra8Unorm` texture into `buffer` (which must have
/// been allocated via [`make_readback_buffer`] with the same `width`/`height`),
/// then map and return the unpadded `width * height * 4` bytes.  Buffer is
/// unmapped on the way out so callers can reuse it on the next call.
pub fn read_into(
    ctx: &GpuContext,
    texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, GpuError> {
    let padded_bytes_per_row = padded_row_bytes(width);

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rpview-gpu readback encoder"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    ctx.queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    ctx.device
        .poll(wgpu::PollType::Wait)
        .map_err(|e| GpuError::DevicePoll(format!("{e:?}")))?;
    rx.recv()
        .map_err(|e| GpuError::BufferMap(format!("recv: {e}")))?
        .map_err(|e| GpuError::BufferMap(format!("map: {e:?}")))?;

    let mapped = slice.get_mapped_range();
    let row_bytes = (width * 4) as usize;
    let padded_row_bytes = padded_bytes_per_row as usize;
    let mut out = Vec::with_capacity(row_bytes * height as usize);
    for row in 0..height as usize {
        let start = row * padded_row_bytes;
        out.extend_from_slice(&mapped[start..start + row_bytes]);
    }
    drop(mapped);
    buffer.unmap();
    Ok(out)
}
