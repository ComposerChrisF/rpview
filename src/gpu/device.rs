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
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("rpview-gpu"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
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
