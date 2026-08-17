//! Orthographic 2D camera with pan/zoom.
//!
//! World coordinates: pixels of the Azgaar canvas (top-left origin,
//! +Y pointing down). Orthographic 2D projection without rotation (plan
//! sec.9.3). Pan moves `center`, zoom scales `extent` relative to the pixel
//! under the cursor (zoom-to-cursor) for a natural feel.
//!
//! Think of the model like this:
//! - `center` = world point at the center of the window.
//! - `extent_y` = visible world height (in pixels). The visible width derives
//!   from the viewport aspect: `extent_x = extent_y * aspect`.
//! - Zoom = changing `extent_y`; a smaller visible height means more zoom.
//!
//! We upload a `view_proj` matrix (column-major 4x4, `f32`) to the GPU.

use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// Camera uniform buffer, sent to the shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
pub struct CameraUniform {
    /// `view_proj` column-major, compatible with WGSL (`mat4x4<f32>`).
    pub view_proj: [f32; 16],
}

/// Orthographic 2D camera.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Camera center in world coordinates (pixels).
    pub center: [f32; 2],
    /// Viewport height in world pixels. Width = `extent_y * aspect`.
    pub extent_y: f32,
    /// Viewport aspect (width / height in surface pixels).
    pub aspect: f32,
    /// Map bounds (min_x, min_y, max_x, max_y). Set with `frame_bounds`.
    pub bounds_min: [f32; 2],
    pub bounds_max: [f32; 2],
}

impl Camera {
    /// Builds with sensible defaults: center at origin, visible height 1000px,
    /// aspect derived from the window.
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
            bounds_min: [0.0, 0.0],
            bounds_max: [1000.0, 1000.0],
        }
    }

    /// Updates the aspect when the window size changes.
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

    /// Orthographic view*proj matrix (clip space [-1,1] x [-1,1], Y up in NDC).
    ///
    /// We map world (+Y down) to NDC (+Y up) by inverting Y explicitly so the
    /// geometry is not vertically mirrored when rendering.
    pub fn view_proj(&self) -> Mat4 {
        let ex = self.extent_x();
        let ey = self.extent_y;
        let (cx, cy) = (self.center[0], self.center[1]);
        let left = cx - ex * 0.5;
        let right = cx + ex * 0.5;
        let bottom = cy + ey * 0.5; // +Y down in world
        let top = cy - ey * 0.5; // -Y up in world
        Mat4::orthographic_rh(left, right, bottom, top, -1.0, 1.0)
    }

    /// Packs the matrix for the uniform buffer (column-major `f32x16`).
    pub fn uniform(&self) -> CameraUniform {
        let m = self.view_proj();
        let cols = m.to_cols_array();
        CameraUniform { view_proj: cols }
    }

    /// Converts a world coordinate (pixels, +Y down, top-left origin) to a
    /// screen pixel.
    pub fn world_to_screen(&self, world: [f32; 2], surface_size: [f32; 2]) -> [f32; 2] {
        let (sw, sh) = (surface_size[0], surface_size[1]);
        if sw <= 0.0 || sh <= 0.0 {
            return [0.0, 0.0];
        }
        let nx = (world[0] - self.center[0]) / self.extent_x() + 0.5;
        let ny = (world[1] - self.center[1]) / self.extent_y + 0.5;
        [nx * sw, ny * sh]
    }

    /// Converts a screen coordinate (surface pixels, +Y down, top-left origin)
    /// to a world coordinate (pixels, +Y down, top-left origin).
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

    /// Zoom preserving the world point under `cursor_screen`.
    /// `factor > 1.0` zooms in (`extent_y` shrinks); `< 1.0` zooms out.
    pub fn zoom_at_cursor(&mut self, cursor_screen: [f32; 2], surface_size: [f32; 2], factor: f32) {
        let world_before = self.screen_to_world(cursor_screen, surface_size);
        self.extent_y = (self.extent_y / factor).clamp(Self::MIN_EXTENT, Self::MAX_EXTENT);
        let world_after = self.screen_to_world(cursor_screen, surface_size);
        self.center[0] += world_before[0] - world_after[0];
        self.center[1] += world_before[1] - world_after[1];
        self.constrain();
    }

    /// Pan by a delta in surface pixels (dragging the map).
    pub fn pan_by_screen_delta(&mut self, delta_px: [f32; 2], surface_size: [f32; 2]) {
        if surface_size[0] <= 0.0 || surface_size[1] <= 0.0 {
            return;
        }
        let frac_x = delta_px[0] / surface_size[0];
        let frac_y = delta_px[1] / surface_size[1];
        self.center[0] -= frac_x * self.extent_x();
        self.center[1] -= frac_y * self.extent_y;
        self.constrain();
    }

    /// Centers the camera on a world bounding box (pixels) for the initial framing.
    pub fn frame_bounds(&mut self, min: [f32; 2], max: [f32; 2]) {
        self.bounds_min = min;
        self.bounds_max = max;
        if !min[0].is_finite() || !min[1].is_finite() || max[0] <= min[0] || max[1] <= min[1] {
            self.bounds_min = [0.0, 0.0];
            self.bounds_max = [1000.0, 1000.0];
        }
        let cx = (self.bounds_min[0] + self.bounds_max[0]) * 0.5;
        let cy = (self.bounds_min[1] + self.bounds_max[1]) * 0.5;
        let w = (self.bounds_max[0] - self.bounds_min[0]).max(1.0);
        let h = (self.bounds_max[1] - self.bounds_min[1]).max(1.0);
        // Small padding so coastline fractal smearing is not clipped at the
        // window edge, but keep it tight so the map fills the screen (cols of
        // ocean beyond the world edge are not part of the map).
        let pad = 1.03;
        let w_needed = w * pad;
        let h_needed = h * pad;
        let h_for_w = w_needed / self.aspect.max(1e-6);
        // Fit to cover the whole viewport (like Azgaar's `fitMapToScreen`):
        // pick the smaller visible height so the map fills the window entirely,
        // cropping the axis that does not match, instead of leaving letterbox
        // (empty, gray) bands outside the world.
        let extent_y = h_needed.min(h_for_w);
        self.center = [cx, cy];
        self.extent_y = extent_y.clamp(Self::MIN_EXTENT, Self::MAX_EXTENT);
        self.constrain();
    }

    /// Restrains center and zoom so the map cannot be lost.
    fn constrain(&mut self) {
        let half_w = self.extent_x() * 0.5;
        let half_h = self.extent_y * 0.5;
        let map_w = self.bounds_max[0] - self.bounds_min[0];
        let map_h = self.bounds_max[1] - self.bounds_min[1];
        let margin_x = map_w * 0.3;
        let margin_y = map_h * 0.3;

        if half_w < map_w * 0.5 + margin_x {
            self.center[0] = self.center[0].clamp(
                self.bounds_min[0] - margin_x + half_w,
                self.bounds_max[0] + margin_x - half_w,
            );
        } else {
            self.center[0] = (self.bounds_min[0] + self.bounds_max[0]) * 0.5;
        }
        if half_h < map_h * 0.5 + margin_y {
            self.center[1] = self.center[1].clamp(
                self.bounds_min[1] - margin_y + half_h,
                self.bounds_max[1] + margin_y - half_h,
            );
        } else {
            self.center[1] = (self.bounds_min[1] + self.bounds_max[1]) * 0.5;
        }

        let min_ext = (map_w.min(map_h) * 0.005).max(4.0);
        // The map should never shrink beyond roughly fitting the window. More
        // zoom-out only lets the user see endless blue void outside the world,
        // which is meaningless (Azgaar has no ocean beyond the world edge).
        let max_ext = map_w.max(map_h).max(min_ext) * 1.2;
        self.extent_y = self.extent_y.clamp(min_ext, max_ext);
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
        // extent_y should have been reduced to half.
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
        // aspect 2: bbox 2000x1000 padded (1.03) -> h_needed=1030, w_needed=2060 -> h_for_w=1030. extent_y=1030
        assert!((cam.extent_y - 1030.0).abs() < 1e-3);
    }
}
