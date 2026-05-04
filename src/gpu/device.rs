//! Standalone wgpu instance, adapter, device, and queue. Lives independently
//! of GPUI's blade-graphics context — two GPU contexts in one process is safe
//! on Metal and D3D12 (the driver serializes access).

use std::sync::OnceLock;
use thiserror::Error;
use wgpu::{Adapter, Device, Instance, Queue};

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no compatible GPU adapter found")]
    NoAdapter,
    #[error("device request failed: {0}")]
    DeviceRequest(String),
    #[error("buffer map failed: {0}")]
    BufferMap(String),
    #[error("device poll failed: {0}")]
    DevicePoll(String),
    /// Requested output dimensions exceed the device's max texture
    /// dimension.  Surfaced as `Err` from `process_pipeline` so the worker
    /// can fall back gracefully — much better than letting wgpu's
    /// `create_texture` panic deep inside the rayon thread.
    #[error("output {width}×{height} exceeds GPU max texture dimension {max}")]
    OutputTooLarge { width: u32, height: u32, max: u32 },
}

pub struct GpuContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

static GPU_CONTEXT: OnceLock<Option<GpuContext>> = OnceLock::new();

/// Returns the lazily-initialized GPU context, or `None` if no compatible
/// adapter was available at first-call time. Subsequent calls return the
/// same result — we don't retry on failure.
pub fn get_context() -> Option<&'static GpuContext> {
    GPU_CONTEXT
        .get_or_init(|| match try_init() {
            Ok(ctx) => Some(ctx),
            Err(e) => {
                eprintln!("[gpu] init failed: {e}; GPU filters disabled");
                None
            }
        })
        .as_ref()
}

fn try_init() -> Result<GpuContext, GpuError> {
    let instance = Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .map_err(|_| GpuError::NoAdapter)?;
    // Use the adapter's full limits rather than `Limits::default()` (which
    // caps at the conservative WebGPU spec: 8192 textures, 256 MB buffers).
    // Apple M-series supports 16384 textures and multi-GB buffers — that
    // headroom is needed for legitimate large operations like 4× upscale of
    // a multi-MP image.  Without this, `create_texture` panics on
    // validation failure deep in the rayon worker (see crash report
    // 2026-05-04 13:05).
    let adapter_limits = adapter.limits();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("rpview-gpu"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter_limits,
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|e| GpuError::DeviceRequest(e.to_string()))?;
    Ok(GpuContext {
        instance,
        adapter,
        device,
        queue,
    })
}
