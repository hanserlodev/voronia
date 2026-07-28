pub fn show(ui: &mut egui::Ui) {
    ui.heading("World config");
    ui.label("(requires regeneration)");
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Seed:");
        ui.add_enabled(false, egui::TextEdit::singleline(&mut String::new()));
    });
    ui.horizontal(|ui| {
        ui.label("Points:");
        ui.add_enabled(false, egui::TextEdit::singleline(&mut String::new()));
    });
    ui.separator();
    ui.heading("Preferences");
    ui.label("Autosave");
    ui.label("Language");
    ui.separator();
    ui.label("Coming in future phases.");
}
