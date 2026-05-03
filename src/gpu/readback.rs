//! GPU → CPU readback for RGBA8/BGRA8 textures. Handles the 256-byte row
//! alignment that wgpu requires for buffer copies and strips it on the way out.

use crate::gpu::device::{GpuContext, GpuError};

const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Copy an `Rgba8Unorm`/`Bgra8Unorm` texture out to a `Vec<u8>` of `width * height * 4`
/// bytes, stripping the per-row padding the GPU required during the copy.
pub fn read_texture_8bpp(
    ctx: &GpuContext,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, GpuError> {
    let bytes_per_pixel = 4u32;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let padding = (ROW_ALIGN - unpadded_bytes_per_row % ROW_ALIGN) % ROW_ALIGN;
    let padded_bytes_per_row = unpadded_bytes_per_row + padding;
    let buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);

    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rpview-gpu readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

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
            buffer: &buffer,
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
    let row_bytes = unpadded_bytes_per_row as usize;
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
