//! Internal pipeline machinery for the unified OKLab filter pipeline:
//!   source upload, OKLab buffer allocation, decode/encode passes, per-stage
//!   compute dispatch, bind group layouts, lazily-cached pipelines.
//!
//! Public surface lives in `crate::gpu::unified`.

use std::sync::OnceLock;

use crate::gpu::OKLAB_WGSL;
use crate::gpu::device::GpuContext;

const WORKGROUP: u32 = 16;

pub fn dispatch_count(width: u32, height: u32) -> (u32, u32, u32) {
    (width.div_ceil(WORKGROUP), height.div_ceil(WORKGROUP), 1)
}

// --- Texture allocation ---------------------------------------------------

pub fn upload_source_srgb(
    ctx: &GpuContext,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> wgpu::Texture {
    let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rpview-gpu source srgb"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    ctx.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture
}

/// OKLab working buffer: rgba16float storing (L, a, b, alpha) per pixel.
pub fn make_oklab_buffer(ctx: &GpuContext, width: u32, height: u32) -> wgpu::Texture {
    ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rpview-gpu oklab buffer"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

/// Final BGRA output (rgba8unorm with channel swap done in encode shader).
pub fn make_bgra_output(ctx: &GpuContext, width: u32, height: u32) -> wgpu::Texture {
    ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rpview-gpu bgra output"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

// --- Bind group layouts ---------------------------------------------------

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn storage_entry(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Decode bind layout: srgb input + rgba16float storage out (no uniform).
pub fn decode_layout(ctx: &GpuContext) -> &'static wgpu::BindGroupLayout {
    static L: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
    L.get_or_init(|| {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rpview-gpu decode layout"),
                entries: &[
                    texture_entry(0),
                    storage_entry(1, wgpu::TextureFormat::Rgba16Float),
                ],
            })
    })
}

/// Encode bind layout: rgba16float input + rgba8unorm storage out (no uniform).
pub fn encode_layout(ctx: &GpuContext) -> &'static wgpu::BindGroupLayout {
    static L: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
    L.get_or_init(|| {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rpview-gpu encode layout"),
                entries: &[
                    texture_entry(0),
                    storage_entry(1, wgpu::TextureFormat::Rgba8Unorm),
                ],
            })
    })
}

/// Per-stage filter layout: rgba16float input + rgba16float storage out + uniform.
pub fn stage_layout(ctx: &GpuContext) -> &'static wgpu::BindGroupLayout {
    static L: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
    L.get_or_init(|| {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rpview-gpu stage layout"),
                entries: &[
                    texture_entry(0),
                    storage_entry(1, wgpu::TextureFormat::Rgba16Float),
                    uniform_entry(2),
                ],
            })
    })
}

// --- Pipeline construction ------------------------------------------------

fn build_pipeline(
    ctx: &GpuContext,
    label: &str,
    body: &str,
    with_oklab: bool,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::ComputePipeline {
    let mut source = String::new();
    if with_oklab {
        source.push_str(OKLAB_WGSL);
        source.push('\n');
    }
    source.push_str(body);
    let module = ctx
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[layout],
            push_constant_ranges: &[],
        });
    ctx.device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
}

pub fn build_stage_pipeline(ctx: &GpuContext, label: &str, body: &str) -> wgpu::ComputePipeline {
    build_pipeline(ctx, label, body, false, stage_layout(ctx))
}

pub fn decode_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        build_pipeline(
            ctx,
            "rpview-gpu decode",
            include_str!("shaders/decode_oklab.wgsl"),
            true,
            decode_layout(ctx),
        )
    })
}

pub fn encode_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        build_pipeline(
            ctx,
            "rpview-gpu encode",
            include_str!("shaders/encode_srgb_bgra.wgsl"),
            true,
            encode_layout(ctx),
        )
    })
}

// --- Uniform buffer + dispatch helpers ------------------------------------

pub fn make_uniform_buffer(ctx: &GpuContext, bytes: &[u8]) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    ctx.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rpview-gpu uniforms"),
            contents: bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
}

/// Dispatch the decode pass (sRGB source → OKLab buffer).
pub fn encode_decode(
    ctx: &GpuContext,
    encoder: &mut wgpu::CommandEncoder,
    source: &wgpu::Texture,
    oklab: &wgpu::Texture,
    width: u32,
    height: u32,
) {
    let src_view = source.create_view(&wgpu::TextureViewDescriptor::default());
    let dst_view = oklab.create_view(&wgpu::TextureViewDescriptor::default());
    let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rpview-gpu decode bind"),
        layout: decode_layout(ctx),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&src_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&dst_view),
            },
        ],
    });
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("rpview-gpu decode pass"),
        timestamp_writes: None,
    });
    pass.set_pipeline(decode_pipeline(ctx));
    pass.set_bind_group(0, &bind, &[]);
    let (gx, gy, gz) = dispatch_count(width, height);
    pass.dispatch_workgroups(gx, gy, gz);
}

/// Dispatch the encode pass (OKLab buffer → BGRA8 output).
pub fn encode_encode(
    ctx: &GpuContext,
    encoder: &mut wgpu::CommandEncoder,
    oklab: &wgpu::Texture,
    output: &wgpu::Texture,
    width: u32,
    height: u32,
) {
    let src_view = oklab.create_view(&wgpu::TextureViewDescriptor::default());
    let dst_view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rpview-gpu encode bind"),
        layout: encode_layout(ctx),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&src_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&dst_view),
            },
        ],
    });
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("rpview-gpu encode pass"),
        timestamp_writes: None,
    });
    pass.set_pipeline(encode_pipeline(ctx));
    pass.set_bind_group(0, &bind, &[]);
    let (gx, gy, gz) = dispatch_count(width, height);
    pass.dispatch_workgroups(gx, gy, gz);
}

/// Dispatch a per-stage filter (OKLab → OKLab, with uniform parameters).
pub fn encode_stage(
    ctx: &GpuContext,
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    src: &wgpu::Texture,
    dst: &wgpu::Texture,
    uniform: &wgpu::Buffer,
    width: u32,
    height: u32,
    label: &str,
) {
    let src_view = src.create_view(&wgpu::TextureViewDescriptor::default());
    let dst_view = dst.create_view(&wgpu::TextureViewDescriptor::default());
    let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: stage_layout(ctx),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&src_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&dst_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
        ],
    });
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some(label),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind, &[]);
    let (gx, gy, gz) = dispatch_count(width, height);
    pass.dispatch_workgroups(gx, gy, gz);
}
