use vor_core::World;

pub fn show(ui: &mut egui::Ui, _world: &mut World, _dirty: &mut bool) {
    ui.heading("Edit");
    ui.label("State");
    ui.label("Burg");
    ui.label("Province");
    ui.label("River");
    ui.label("Culture");
    ui.separator();

    ui.heading("Regenerate");
    ui.label("States");
    ui.label("Burgs");
    ui.label("Rivers");
    ui.separator();

    ui.heading("Add");
    ui.label("Burg");
    ui.label("River");
    ui.separator();

    ui.heading("Show");
    ui.label("Cells (ids)");
    ui.label("Charts");
    ui.label("Minimap");
}
