//! Exporta el frame actual del mapa como PNG (snapshot de GPU a disco).
//!
//! Renderiza todas las capas activas a una textura offscreen del tamaño
//! especificado, la lee de vuelta a CPU y la encodea como PNG.

use std::path::Path;
use std::sync::mpsc;

use anyhow::{Context, Result};
use vor_render::{Camera, LayerFlags, Renderer};

/// Renderiza el mapa a un PNG del tamaño especificado.
///
/// `surface_size` es el tamaño actual de la ventana (para cámara y formato).
/// `export_width`/`export_height` es la resolución de salida.
/// La cámara se usa tal cual (misma vista que el viewport actual).
pub fn export_png(
    renderer: &Renderer,
    camera: &Camera,
    layer_flags: &LayerFlags,
    export_width: u32,
    export_height: u32,
    path: &Path,
) -> Result<()> {
    let format = renderer.format;
    let device = &renderer.device;
    let queue = &renderer.queue;

    // Offscreen texture: mismo formato que la surface (RENDER_ATTACHMENT + COPY_SRC)
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vor-export-png-tex"),
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Calcular camera uniform para el tamaño de exportación (misma vista, distinto aspect)
    // Necesito recrear la cámara con el nuevo viewport o escalar el uniform manual.
    // La forma más limpia: generar el uniform desde la cámara actual.
    let uniform = camera.uniform();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("vor-export-png-encoder"),
    });

    // Escribir el uniform (la pos/viewport de cámara no cambia, solo el aspect
    // del render target. Para mantener la misma vista, reajustamos la cámara
    // temporalmente con el aspect nuevo.)
    // En vez de eso, mejor crear una Camera temporal con el aspect del export.
    let mut export_camera = *camera;
    export_camera.set_viewport(export_width, export_height);
    let export_uniform = export_camera.uniform();
    queue.write_buffer(
        &renderer.camera_buf,
        0,
        bytemuck::cast_slice(&[export_uniform]),
    );

    // Render pass a offscreen texture
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vor-export-png-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
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
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for layer_idx in layer_flags.active_indices() {
            renderer.draw_layer(&mut pass, layer_idx);
        }
    }

    // Restaurar uniform original
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
            texture: &texture,
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

    // Map y leer
    let buffer_slice = readback_buf.slice(..);
    let (tx, rx) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .context("canal de map_async roto")?
        .context("map_async falló")?;

    let data: Vec<u8> = {
        let mapped = buffer_slice.get_mapped_range();
        let raw = mapped.to_vec();
        drop(mapped);
        readback_buf.unmap();
        raw
    };

    // Convertir BGRA → RGBA si el formato de surface es BGRA
    let rgba: Vec<u8> = if format == wgpu::TextureFormat::Bgra8Unorm
        || format == wgpu::TextureFormat::Bgra8UnormSrgb
    {
        data.chunks_exact(4)
            .flat_map(|px| [px[2], px[1], px[0], px[3]])
            .collect()
    } else {
        data
    };

    let img = image::RgbaImage::from_raw(export_width, export_height, rgba)
        .context("image::from_raw falló")?;
    img.save(path)
        .with_context(|| format!("guardar PNG en {} falló", path.display()))?;

    // Re-escribir uniform original (por si acaso)
    queue.write_buffer(&renderer.camera_buf, 0, bytemuck::cast_slice(&[uniform]));

    Ok(())
}
