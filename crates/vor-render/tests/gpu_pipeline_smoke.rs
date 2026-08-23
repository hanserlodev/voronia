//! GPU smoke test: creates a headless device and instantiates every new
//! overlay pipeline added during the Landmass parity round (relief atlas,
//! ocean pattern, masked texture with shift). WGSL is validated when the
//! shader module is created and the pipeline layout when the pipeline is
//! built — `cargo check` cannot catch these.
//!
//! Soft-skips if no GPU adapter is available (e.g. CI without lavapipe).

use wgpu::util::DeviceExt;

fn headless_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).ok()
}

/// The renderer's camera group layout: a single `mat4x4<f32>` uniform at
/// group 0, binding 0 (mirrors `Renderer::camera_bind_layout`).
fn camera_layout(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("smoke-camera-bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("smoke-camera-buf"),
        contents: &[0u8; 64],
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("smoke-camera-bg"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buf.as_entire_binding(),
        }],
    });
    (layout, bind)
}

#[test]
fn new_overlay_pipelines_build_headless() {
    let Some((device, queue)) = headless_device() else {
        eprintln!("no GPU adapter available — skipping smoke test");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let (cam_layout, _cam_bind) = camera_layout(&device);

    // 1. Relief atlas overlay (new WGSL + quad mesh from instances).
    let icons = vec![
        vor_render::relief::ReliefIcon {
            symbol: 0,
            x: 10.0,
            y: 10.0,
            s: 5.0,
        },
        vor_render::relief::ReliefIcon {
            symbol: 3,
            x: 40.0,
            y: 40.0,
            s: 4.0,
        },
    ];
    let atlas = vec![128u8; 768 * 768 * 4];
    let relief = vor_render::ReliefIconsOverlay::new(
        &device,
        &queue,
        format,
        768,
        768,
        &atlas,
        1, // no MSAA offscreen
        &cam_layout,
        &icons,
    );
    let _ = format!("{:?}", relief);

    // 2. Ocean pattern overlay (Repeat tiling + embedded opacity).
    let _pattern = vor_render::OceanPatternOverlay::new(
        &device,
        &queue,
        format,
        64,
        64,
        &vec![255u8; 64 * 64 * 4],
        1,
        [0.0, 0.0],
        [1000.0, 800.0],
        &cam_layout,
        0.2,
    );

    // 3. Texture overlay with the shift uniform (stencil mask test state).
    let _tex = vor_render::TextureOverlay::new(
        &device,
        &queue,
        format,
        64,
        64,
        &vec![255u8; 64 * 64 * 4],
        1,
        [0.0, 0.0],
        [1000.0, 800.0],
        &cam_layout,
    );
}
