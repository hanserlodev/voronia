use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use tracing::{trace, warn};
use wgpu::{Device, MultisampleState, Queue, TextureFormat};

/// GPU text system using glyphon.
///
/// Owns the font system, texture atlas, glyph cache, and viewport. Follows the
/// glyphon example pattern: `prepare` BEFORE the render pass, `render` inside
/// the render pass.
/// A screen-space text label: text + position + size + color.
#[derive(Debug, Clone)]
pub struct Label {
    pub text: String,
    /// Screen X (left edge), surface pixels.
    pub x: f32,
    /// Screen Y (top edge), surface pixels.
    pub y: f32,
    /// Font size in surface pixels (scaled by caller when the camera zooms).
    pub font_size: f32,
    /// Linear RGBA color.
    pub color_rgba: [f32; 4],
}

impl Label {
    /// Creates a label with the given screen position/size/color.
    pub fn new(
        text: impl Into<String>,
        x: f32,
        y: f32,
        font_size: f32,
        color_rgba: [f32; 4],
    ) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            font_size,
            color_rgba,
        }
    }
}

pub struct TextSystem {
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
    pub viewport: Viewport,
    pub renderer: TextRenderer,
    renderer_no_msaa: TextRenderer,
    buffers: Vec<Buffer>,
    pending_draw: bool,
}

impl TextSystem {
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        surface_size: (u32, u32),
        msaa_count: u32,
    ) -> Self {
        let cache = Cache::new(device);
        let mut font_system = FontSystem::new();
        let font_count = font_system.db().len();
        trace!("glyphon FontSystem loaded with {font_count} font faces");
        let swash_cache = SwashCache::new();
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let mut viewport = Viewport::new(device, &cache);

        let renderer = TextRenderer::new(
            &mut atlas,
            device,
            MultisampleState {
                count: msaa_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // The MSAA renderer draws inside the map pass, which now carries a
            // depth-stencil attachment (the landmask buffer). Its pipeline must
            // declare a matching depth-stencil state or wgpu rejects the draw.
            Some(crate::renderer::stencil_passthrough()),
        );
        let renderer_no_msaa =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        let metrics = Metrics::new(32.0, 42.0);
        let buffer = Buffer::new(&mut font_system, metrics);

        viewport.update(
            queue,
            Resolution {
                width: surface_size.0.max(1),
                height: surface_size.1.max(1),
            },
        );

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderer,
            renderer_no_msaa,
            buffers: vec![buffer],
            pending_draw: false,
        }
    }

    pub fn resize(&mut self, queue: &Queue, width: u32, height: u32) {
        self.viewport.update(
            queue,
            Resolution {
                width: width.max(1),
                height: height.max(1),
            },
        );
    }

    /// Prepares one or more text labels for rendering.
    ///
    /// Each label is a separate line with its own buffer, so many independent
    /// snippets (grid labels, burgs, rivers...) can be batched in a single
    /// GPU prepare. Must be called OUTSIDE the render pass (before
    /// `begin_render_pass`) because it uploads glyphs via `queue.write_texture`
    /// and `queue.write_buffer`.
    pub fn prepare(&mut self, device: &Device, queue: &Queue, labels: &[Label]) {
        self.buffers.clear();

        for label in labels {
            let buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(label.font_size, label.font_size * 1.3),
            );
            self.buffers.push(buffer);
        }

        // Populate each buffer's text (layout/shape happen at prepare time).
        for (i, label) in labels.iter().enumerate() {
            let buffer = &mut self.buffers[i];
            buffer.set_size(
                &mut self.font_system,
                Some(8192.0),
                Some(label.font_size * 1.3),
            );
            buffer.set_text(
                &mut self.font_system,
                &label.text,
                Attrs::new().family(Family::SansSerif),
                Shaping::Basic,
            );
            buffer.shape_until_scroll(&mut self.font_system, true);
        }

        let glyphs: usize = self
            .buffers
            .iter()
            .flat_map(|b| b.layout_runs())
            .map(|r| r.glyphs.len())
            .sum();
        trace!(
            "glyphon prepare: {} labels, {} glyphs total",
            labels.len(),
            glyphs
        );

        // Prepare both renderers (MSAA and non-MSAA) so both have populated
        // glyph_vertices. `prepare` consumes the `TextArea` slice by value, so
        // build it fresh for each renderer.
        let mut ok = true;
        for (label, renderer) in [
            ("msaa", &mut self.renderer),
            ("no-msaa", &mut self.renderer_no_msaa),
        ] {
            let areas: Vec<TextArea<'_>> = labels
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let [r, g, b, a] = l.color_rgba;
                    let color = Color::rgba(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        (a * 255.0) as u8,
                    );
                    TextArea {
                        buffer: &self.buffers[i],
                        left: l.x,
                        top: l.y,
                        scale: 1.0,
                        bounds: TextBounds::default(),
                        default_color: color,
                        custom_glyphs: &[],
                    }
                })
                .collect();
            match renderer.prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash_cache,
            ) {
                Ok(()) => {}
                Err(e) => {
                    warn!("glyphon prepare ({label}) error: {e}");
                    ok = false;
                }
            }
        }
        self.pending_draw = ok;
    }

    /// Renders to the active render pass (MSAA, the same one as the map).
    ///
    /// Must be called INSIDE the render pass.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if !self.pending_draw {
            trace!("glyphon render skipped (no pending draw)");
            return;
        }
        trace!("glyphon rendering...");
        if let Err(e) = self
            .renderer
            .render(&self.atlas, &self.viewport, &mut *pass)
        {
            warn!("glyphon render error: {e}");
        }
    }

    /// DEBUG: renders text to an external encoder using a pass without MSAA,
    /// directly onto the resolve_view (surface). Useful for diagnosing whether
    /// the issue is the MSAA pass.
    pub fn render_debug_no_msaa<'pass>(
        &'pass self,
        encoder: &'pass mut wgpu::CommandEncoder,
        resolve_view: &'pass wgpu::TextureView,
    ) {
        if !self.pending_draw {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vor-text-debug"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: resolve_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        trace!("glyphon render (debug no-msaa)...");
        if let Err(e) = self
            .renderer_no_msaa
            .render(&self.atlas, &self.viewport, &mut pass)
        {
            warn!("glyphon render error (no-msaa): {e}");
        }
    }

    pub fn trim(&mut self) {
        self.atlas.trim();
        self.pending_draw = false;
    }
}
