//! Exports the current map frame as PNG (GPU snapshot to disk).
//!
//! Renders all active layers to an offscreen texture of the specified size,
//! reads it back to the CPU and encodes it as PNG.

use std::path::Path;
use std::sync::mpsc;

use anyhow::{Context, Result};
use vor_render::{layers::DynamicLayerIds, Camera, LayerFlags, Renderer};

/// Renders the map to a PNG of the specified size.
///
/// `surface_size` is the current window size (for camera and format).
/// `export_width`/`export_height` is the output resolution.
/// The camera is used as-is (same view as the current viewport).
#[allow(clippy::too_many_arguments)]
pub fn export_png(
    renderer: &Renderer,
    camera: &Camera,
    layer_flags: &LayerFlags,
    dyn_ids: &DynamicLayerIds,
    export_width: u32,
    export_height: u32,
    path: &Path,
) -> Result<()> {
    let format = renderer.format;
    let device = &renderer.device;
    let queue = &renderer.queue;

    // Offscreen texture resolve target: single-sample, COPY_SRC for readback
    let resolve_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vor-export-png-resolve"),
        size: wgpu::Extent3d {
            width: export_width,
            height: export_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let resolve_view = resolve_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Offscreen MSAA 4x texture (render target)
    let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vor-export-png-msaa"),
        size: wgpu::Extent3d {
            width: export_width,
            height: export_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: renderer.msaa_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Masked layers use the same stencil contract as the interactive renderer:
    // layer 0 writes stencil=1 and masked layers test it. Without an offscreen
    // stencil attachment the PNG path either fails validation or silently
    // diverges from the on-screen composition.
    let stencil_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vor-export-png-stencil"),
        size: wgpu::Extent3d {
            width: export_width,
            height: export_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: renderer.msaa_count,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24PlusStencil8,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let stencil_view = stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Compute the camera uniform for the export size (same view, different aspect).
    // Either recreate the camera with the new viewport or scale the uniform manually.
    // The cleanest way: generate the uniform from the current camera.
    let uniform = camera.uniform();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("vor-export-png-encoder"),
    });

    // Write the uniform (the camera pos/viewport does not change, only the
    // aspect of the render target. To keep the same view, we adjust the camera
    // temporarily with the new aspect.)
    // Instead, better to create a temporary Camera with the export aspect.
    let mut export_camera = *camera;
    export_camera.set_viewport(export_width, export_height);
    let export_uniform = export_camera.uniform();
    queue.write_buffer(
        &renderer.camera_buf,
        0,
        bytemuck::cast_slice(&[export_uniform]),
    );

    // Render pass to 4x MSAA offscreen, resolves to resolve_texture
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vor-export-png-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &msaa_view,
                resolve_target: Some(&resolve_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.02,
                        b: 0.05,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Same FMG-ordered sequence as the interactive frame (meshes + line
        // layers interleaved). Line/economy indices arrive via `dyn_ids`.
        // The export uses the same view as the window, so the coastline
        // shadow threshold matches what is on screen.
        let zoom_scale = camera.extent_y.max(1.0) / export_camera.extent_y.max(1.0);
        let draw_opts = vor_render::layers::DrawOptions {
            coastline_shadow: zoom_scale <= vor_render::SHADOW_MAX_SCALE,
        };
        for item in layer_flags.draw_sequence(dyn_ids, &draw_opts) {
            match item {
                vor_render::layers::DrawItem::Mesh(idx) => {
                    renderer.draw_layer(&mut pass, idx);
                }
                vor_render::layers::DrawItem::Line(idx) => {
                    renderer.draw_line_layer(&mut pass, idx);
                }
                // #texture and #terrain overlays live on the app State (own
                // pipelines); exporting them is Phase C work.
                vor_render::layers::DrawItem::Texture => {}
                vor_render::layers::DrawItem::Relief => {}
                vor_render::layers::DrawItem::GoodsIcons => {}
            }
        }
    }

    // Restore original uniform
    queue.write_buffer(&renderer.camera_buf, 0, bytemuck::cast_slice(&[uniform]));

    // Readback buffer
    let bytes_per_row = export_width * 4;
    let buffer_size = (bytes_per_row * export_height) as u64;
    let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vor-export-png-readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture → buffer
    let texel_layout = wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: Some(export_height),
    };
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &resolve_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback_buf,
            layout: texel_layout,
        },
        wgpu::Extent3d {
            width: export_width,
            height: export_height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map and read
    let buffer_slice = readback_buf.slice(..);
    let (tx, rx) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .context("map_async channel broken")?
        .context("map_async failed")?;

    let data: Vec<u8> = {
        let mapped = buffer_slice.get_mapped_range();
        let raw = mapped.to_vec();
        drop(mapped);
        readback_buf.unmap();
        raw
    };

    // Convert BGRA → RGBA if the surface format is BGRA
    let rgba: Vec<u8> = if format == wgpu::TextureFormat::Bgra8Unorm
        || format == wgpu::TextureFormat::Bgra8UnormSrgb
    {
        data.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|px| [px[2], px[1], px[0], px[3]])
            .collect()
    } else {
        data
    };

    let img = image::RgbaImage::from_raw(export_width, export_height, rgba)
        .context("image::from_raw failed")?;
    img.save(path)
        .with_context(|| format!("failed to save PNG to {}", path.display()))?;

    // Re-write the original uniform (just in case)
    queue.write_buffer(&renderer.camera_buf, 0, bytemuck::cast_slice(&[uniform]));

    Ok(())
}
