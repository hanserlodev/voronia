use std::path::Path;

use vor_core::World;
use vor_render::{Camera, LayerFlags, Renderer};

#[allow(clippy::too_many_arguments)]
pub fn export_modal(
    ctx: &egui::Context,
    show: &mut bool,
    renderer: &Renderer,
    camera: &Camera,
    layer_flags: &LayerFlags,
    vorn_save_path: &Path,
    world: &World,
) {
    let mut fmt = 0; // 0=PNG, 1=SVG, 2=JSON
    egui::Window::new("Export")
        .open(show)
        .default_size([280.0, 200.0])
        .show(ctx, |ui| {
            ui.label("Format:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut fmt, 0, "PNG");
                ui.radio_value(&mut fmt, 1, "SVG");
                ui.radio_value(&mut fmt, 2, "JSON");
            });
            ui.separator();
            if ui.button("Export").clicked() {
                match fmt {
                    0 => {
                        let path = vorn_save_path.with_extension("png");
                        match crate::png_export::export_png(
                            renderer,
                            camera,
                            layer_flags,
                            camera.extent_y as u32 * 2,
                            camera.extent_y as u32,
                            &path,
                        ) {
                            Ok(()) => tracing::info!("PNG exported: {}", path.display()),
                            Err(e) => tracing::warn!("PNG failed: {e}"),
                        }
                    }
                    1 => {
                        let path = vorn_save_path.with_extension("svg");
                        match crate::svg_export::export_svg(world, &path) {
                            Ok(()) => tracing::info!("SVG exported: {}", path.display()),
                            Err(e) => tracing::warn!("SVG failed: {e}"),
                        }
                    }
                    2 => {
                        tracing::info!("JSON export not implemented yet");
                    }
                    _ => {}
                }
            }
        });
}

pub fn save_modal(
    ctx: &egui::Context,
    show: &mut bool,
    world: &World,
    vorn_save_path: &Path,
    autosave_enabled: &mut bool,
) {
    egui::Window::new("Save")
        .open(show)
        .default_size([280.0, 160.0])
        .show(ctx, |ui| {
            ui.checkbox(autosave_enabled, "Autosave every 60s");
            if ui.button("Save now").clicked() {
                match vor_format::save::save_world(vorn_save_path, world) {
                    Ok(()) => tracing::info!("Saved: {}", vorn_save_path.display()),
                    Err(e) => tracing::warn!("Save failed: {e}"),
                }
            }
        });
}

pub fn load_modal(ctx: &egui::Context, show: &mut bool) {
    egui::Window::new("Load")
        .open(show)
        .default_size([280.0, 120.0])
        .show(ctx, |ui| {
            ui.label("Formats: .map, .vorn");
            ui.label("File loading via OS dialog coming soon.");
        });
}

pub fn new_map_modal(ctx: &egui::Context, show: &mut bool) {
    egui::Window::new("New Map")
        .open(show)
        .default_size([280.0, 160.0])
        .show(ctx, |ui| {
            ui.label("Seed:");
            ui.label("Width:");
            ui.label("Height:");
            ui.separator();
            ui.label("Coming in future phases.");
        });
}
