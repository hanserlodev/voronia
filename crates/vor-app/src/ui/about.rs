pub fn show(ui: &mut egui::Ui) {
    ui.heading("Voronia");
    ui.label("Fantasy map engine");
    ui.label("Rust + wgpu");
    ui.separator();
    ui.label("Based on Azgaar's");
    ui.label("Fantasy Map Generator");
    ui.label("(MIT)");
    ui.separator();
    ui.hyperlink_to("GitHub", "https://github.com/hanserlodev/voronia");
    ui.label("Open source");
    ui.separator();
    ui.label("By hanserlodev");
}
