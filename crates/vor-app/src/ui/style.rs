use crate::TEXTURES;
use vor_render::temperature::TempUnit;

pub fn show(
    ui: &mut egui::Ui,
    texture_name: &mut String,
    texture_shift: &mut [f32; 2],
    temp_unit: &mut TempUnit,
) {
    ui.heading("Style");
    ui.separator();

    ui.label("Texture:");
    let mut sel_idx = TEXTURES
        .iter()
        .position(|&t| t == texture_name.as_str())
        .unwrap_or(0);
    egui::ComboBox::from_id_salt("texture-select")
        .selected_text(texture_name.as_str())
        .show_ui(ui, |ui| {
            for (i, t) in TEXTURES.iter().enumerate() {
                let label = if *t == "none" {
                    "No texture".to_string()
                } else {
                    t.to_string()
                };
                ui.selectable_value(&mut sel_idx, i, label);
            }
        });
    let new_name = TEXTURES[sel_idx].to_string();
    if *texture_name != new_name {
        *texture_name = new_name;
    }

    // FMG `data-x`/`data-y` shift (style editor sliders).
    ui.horizontal(|ui| {
        ui.label("Shift X:");
        ui.add(egui::Slider::new(&mut texture_shift[0], -100.0..=100.0));
    });
    ui.horizontal(|ui| {
        ui.label("Shift Y:");
        ui.add(egui::Slider::new(&mut texture_shift[1], -100.0..=100.0));
    });
    // FMG `temperatureScale` select (Style editor): unit for isotherm labels.
    ui.horizontal(|ui| {
        ui.label("Temperature scale:");
        egui::ComboBox::from_id_salt("temp-unit-select")
            .selected_text(temp_unit.label())
            .show_ui(ui, |ui| {
                for unit in TempUnit::ALL {
                    ui.selectable_value(temp_unit, unit, unit.label());
                }
            });
    });
}
