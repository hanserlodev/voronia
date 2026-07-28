use vor_render::Camera;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    show_export: &mut bool,
    show_save: &mut bool,
    show_load: &mut bool,
    show_new: &mut bool,
    camera: &mut Camera,
    mesh_bounds_min: [f32; 2],
    mesh_bounds_max: [f32; 2],
    surface_size: &[f32; 2],
) {
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("New").clicked() {
            *show_new = true;
        }
        if ui.button("Export").clicked() {
            *show_export = true;
        }
        if ui.button("Save").clicked() {
            *show_save = true;
        }
        if ui.button("Load").clicked() {
            *show_load = true;
        }
        if ui.button("Reset Zoom").clicked() {
            camera.frame_bounds(mesh_bounds_min, mesh_bounds_max);
            camera.set_viewport(surface_size[0] as u32, surface_size[1] as u32);
        }
    });
}
