//! Camara ortografica 2D con pan/zoom.
//!
//! Coordenadas de mundo: pixels del canvas de Azgaar (origen arriba-izquierda,
//! +Y hacia abajo). Proyeccion ortografica 2D sin rotacion (plan sec.9.3).
//! Pan traslada `center`, zoom escala `extent` relativo al pixel bajo el cursor
//! (zoom-to-cursor) para sensacion natural.
//!
//! Conviene pensar el modelo asi:
//! - `center` = punto de mundo en el centro de la ventana.
//! - `extent_y` = alto de mundo visible (en pixels). El ancho visible se deriva
//!   del aspect del viewport: `extent_x = extent_y * aspect`.
//! - Zoom = cambiar `extent_y`; alto visible mas chico = mas zoom.
//!
//! Pasamos a GPU una matriz `view_proj` (matriz 4x4 column-major, `f32`).

use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// Uniform buffer de la camara, enviada al shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
pub struct CameraUniform {
    /// `view_proj` column-major compatible con WGSL (`mat4x4<f32>`).
    pub view_proj: [f32; 16],
}

/// Camara ortografica 2D.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Centro de la camara en coordenadas de mundo (pixels).
    pub center: [f32; 2],
    /// Alto del viewport en pixels de mundo. Ancho = `extent_y * aspect`.
    pub extent_y: f32,
    /// Aspect del viewport (ancho / alto en pixels de superficie).
    pub aspect: f32,
}

impl Camera {
    /// Construye con defaults razonables: centro en origen, alto visible 1000px, aspect segun ventana.
    pub fn new(center: [f32; 2], extent_y: f32, width: u32, height: u32) -> Self {
        let aspect = if height == 0 {
            1.0
        } else {
            width as f32 / height as f32
        };
        Self {
            center,
            extent_y: extent_y.max(1.0),
            aspect,
        }
    }

    /// Ajusta aspect cuando la ventana cambia de tamano.
    pub fn set_viewport(&mut self, width: u32, height: u32) {
        self.aspect = if height == 0 {
            1.0
        } else {
            width as f32 / height as f32
        };
    }

    #[inline]
    pub fn extent_x(&self) -> f32 {
        self.extent_y * self.aspect
    }

    /// Matriz view*proj ortografica (clip space [-1,1] x [-1,1], Y arriba en NDC).
    ///
    /// Mapeamos mundo (+Y abajo) a NDC (+Y arriba) invirtiendo Y explicitamente
    /// para no tener geometria espejada verticalmente al renderizar.
    pub fn view_proj(&self) -> Mat4 {
        let ex = self.extent_x();
        let ey = self.extent_y;
        let (cx, cy) = (self.center[0], self.center[1]);
        let left = cx - ex * 0.5;
        let right = cx + ex * 0.5;
        let bottom = cy + ey * 0.5; // +Y abajo en mundo
        let top = cy - ey * 0.5; // -Y arriba en mundo
        Mat4::orthographic_rh(left, right, bottom, top, -1.0, 1.0)
    }

    /// Empaqueta la matriz para el uniform buffer (column-major `f32x16`).
    pub fn uniform(&self) -> CameraUniform {
        let m = self.view_proj();
        let cols = m.to_cols_array();
        CameraUniform { view_proj: cols }
    }

    /// Pasa de coordenada de mundo (pixels, +Y abajo, origen TL) a pixel de pantalla.
    pub fn world_to_screen(&self, world: [f32; 2], surface_size: [f32; 2]) -> [f32; 2] {
        let (sw, sh) = (surface_size[0], surface_size[1]);
        if sw <= 0.0 || sh <= 0.0 {
            return [0.0, 0.0];
        }
        let nx = (world[0] - self.center[0]) / self.extent_x() + 0.5;
        let ny = (world[1] - self.center[1]) / self.extent_y + 0.5;
        [nx * sw, ny * sh]
    }

    /// Pasa de coordenada de pantalla (pixels de superficie, +Y abajo, origen TL)
    /// a coordenada de mundo (pixels, +Y abajo, origen TL).
    pub fn screen_to_world(&self, screen_px: [f32; 2], surface_size: [f32; 2]) -> [f32; 2] {
        let (sw, sh) = (surface_size[0], surface_size[1]);
        if sw <= 0.0 || sh <= 0.0 {
            return self.center;
        }
        let nx = screen_px[0] / sw;
        let ny = screen_px[1] / sh;
        let ex = self.extent_x();
        let ey = self.extent_y;
        let world_x = self.center[0] + (nx - 0.5) * ex;
        let world_y = self.center[1] + (ny - 0.5) * ey;
        [world_x, world_y]
    }

    /// Zoom preservando el punto de mundo bajo `cursor_screen`.
    /// `factor > 1.0` acerca (`extent_y` se reduce); `< 1.0` aleja.
    pub fn zoom_at_cursor(&mut self, cursor_screen: [f32; 2], surface_size: [f32; 2], factor: f32) {
        let world_before = self.screen_to_world(cursor_screen, surface_size);
        self.extent_y = (self.extent_y / factor).clamp(Self::MIN_EXTENT, Self::MAX_EXTENT);
        let world_after = self.screen_to_world(cursor_screen, surface_size);
        self.center[0] += world_before[0] - world_after[0];
        self.center[1] += world_before[1] - world_after[1];
    }

    /// Pan por delta en pixels de superficie (arrastrar el mapa).
    pub fn pan_by_screen_delta(&mut self, delta_px: [f32; 2], surface_size: [f32; 2]) {
        if surface_size[0] <= 0.0 || surface_size[1] <= 0.0 {
            return;
        }
        let frac_x = delta_px[0] / surface_size[0];
        let frac_y = delta_px[1] / surface_size[1];
        self.center[0] -= frac_x * self.extent_x();
        self.center[1] -= frac_y * self.extent_y;
    }

    /// Centra la camara en un bounding box de mundo (pixels) para encuadre inicial.
    pub fn frame_bounds(&mut self, min: [f32; 2], max: [f32; 2]) {
        let cx = (min[0] + max[0]) * 0.5;
        let cy = (min[1] + max[1]) * 0.5;
        let w = (max[0] - min[0]).max(1.0);
        let h = (max[1] - min[1]).max(1.0);
        let pad = 1.10;
        let w_needed = w * pad;
        let h_needed = h * pad;
        let h_for_w = w_needed / self.aspect.max(1e-6);
        let extent_y = h_needed.max(h_for_w);
        self.center = [cx, cy];
        self.extent_y = extent_y.clamp(Self::MIN_EXTENT, Self::MAX_EXTENT);
    }

    const MIN_EXTENT: f32 = 4.0;
    const MAX_EXTENT: f32 = 1.0e6_f32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_world_recovers_center() {
        let cam = Camera::new([100.0, 200.0], 10.0, 200, 100);
        let cx = cam.screen_to_world([100.0, 50.0], [200.0, 100.0]);
        assert!((cx[0] - 100.0).abs() < 1e-3);
        assert!((cx[1] - 200.0).abs() < 1e-3);
    }

    #[test]
    fn zoom_in_keeps_cursor_world_pos() {
        let mut cam = Camera::new([0.0, 0.0], 100.0, 400, 400);
        let cursor = [200.0, 200.0];
        let before = cam.screen_to_world(cursor, [400.0, 400.0]);
        cam.zoom_at_cursor(cursor, [400.0, 400.0], 2.0);
        let after = cam.screen_to_world(cursor, [400.0, 400.0]);
        assert!(
            (before[0] - after[0]).abs() < 1e-3,
            "x: {before:?} -> {after:?}"
        );
        assert!(
            (before[1] - after[1]).abs() < 1e-3,
            "y: {before:?} -> {after:?}"
        );
        // extent_y debe haberse reducido a la mitad.
        assert!((cam.extent_y - 50.0).abs() < 1e-3);
    }

    #[test]
    fn pan_moves_center_opposite_to_delta() {
        let mut cam = Camera::new([0.0, 0.0], 100.0, 100, 100);
        // Drag right -> map moves right -> world center moves left.
        cam.pan_by_screen_delta([10.0, 0.0], [100.0, 100.0]);
        assert!((cam.center[0] - (-10.0)).abs() < 1e-3);
    }

    #[test]
    fn frame_bounds_fits_bbox() {
        let mut cam = Camera::new([0.0, 0.0], 1.0, 400, 200);
        cam.frame_bounds([0.0, 0.0], [2000.0, 1000.0]);
        assert!((cam.center[0] - 1000.0).abs() < 1e-3);
        assert!((cam.center[1] - 500.0).abs() < 1e-3);
        // aspect 2: bbox 2000x1000 padded -> h_needed=1100, w_needed=2200 -> h_for_w=1100. extent_y=1100
        assert!((cam.extent_y - 1100.0).abs() < 1e-3);
    }
}
