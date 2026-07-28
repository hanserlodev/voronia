use crate::TEXTURES;

pub fn show(ui: &mut egui::Ui, texture_name: &mut String) {
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
}
