use vor_core::World;
use vor_edit::{EditBuffer, SelectedEntity};

pub fn show(
    ui: &mut egui::Ui,
    picked_cell: Option<usize>,
    world: &mut World,
    edit_buffer: &mut EditBuffer,
    dirty: &mut bool,
) {
    let pop_rate = world.settings.population_rate;

    if let Some(cid) = picked_cell {
        ui.heading(format!("Cell #{cid}"));
        let h = world.pack.cells.height.get(cid).copied().unwrap_or(0);
        let height_m = world.settings.height_m(h);
        ui.label(format!(
            "Height: {height_m:.0}{}",
            world.settings.height_unit
        ));
        let bi = world.pack.cells.biome.get(cid).copied().unwrap_or(0);
        let name = world
            .biomes
            .get(bi as usize)
            .map(|b| b.name.as_str())
            .unwrap_or("?");
        ui.label(format!("Biome: {name}"));

        let sid = world.pack.cells.state.get(cid).copied().unwrap_or(0);
        let sname = if sid > 0 {
            world
                .states
                .iter()
                .find(|s| s.id == sid)
                .map(|s| s.name.as_str())
                .unwrap_or("?")
        } else {
            "Wildlands"
        };
        ui.label(format!("State: {sname}"));

        let ci = world.pack.cells.culture.get(cid).copied().unwrap_or(0);
        let cname = if ci > 0 {
            world
                .cultures
                .get(ci as usize)
                .map(|c| c.name.as_str())
                .unwrap_or("?")
        } else {
            "Wildlands"
        };
        ui.label(format!("Culture: {cname}"));

        let pid = world.pack.cells.province.get(cid).copied().unwrap_or(0);
        let pname = if pid > 0 {
            world
                .provinces
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.name.as_str())
                .unwrap_or("?")
        } else {
            "\u{2014}"
        };
        ui.label(format!("Province: {pname}"));

        let bid = world.pack.cells.burg.get(cid).copied().unwrap_or(0);
        let bname = if bid > 0 {
            world
                .burgs
                .iter()
                .find(|b| b.id == bid)
                .map(|b| b.name.as_str())
                .unwrap_or("?")
        } else {
            "\u{2014}"
        };
        ui.label(format!("Burg: {bname}"));

        let rid = world.pack.cells.river.get(cid).copied().unwrap_or(0);
        let rname = if rid > 0 {
            world
                .rivers
                .iter()
                .find(|r| r.id == rid)
                .map(|r| r.name.as_str())
                .unwrap_or("?")
        } else {
            "\u{2014}"
        };
        ui.label(format!("River: {rname}"));

        let pop = world.pack.cells.population.get(cid).copied().unwrap_or(0.0);
        ui.label(format!("Population: {:.0}", pop * pop_rate));

        ui.separator();
        ui.heading("Editor");
        let sel = entity_from_cell(world, cid);
        if edit_buffer.selected_entity_id != sel {
            edit_buffer.selected_entity_id = sel;
            edit_buffer.load_entity(world);
        }
        if let Some(ent) = edit_buffer.selected_entity_id {
            match ent {
                SelectedEntity::State(id) => {
                    ui.label(format!("State #{id}"));
                    rename_field(ui, "name", &mut edit_buffer.rename_buffer);
                    if edit_buffer.rename_buffer.is_empty() {
                        edit_buffer.load_entity(world);
                    }
                    if ui.button("Apply name").clicked() {
                        _ = vor_edit::rename_state(world, id, &edit_buffer.rename_buffer);
                        *dirty = true;
                    }
                    color_field(ui, "color", &mut edit_buffer.color_buffer);
                    if ui.button("Apply color").clicked() {
                        _ = vor_edit::set_state_color(world, id, &edit_buffer.color_buffer);
                        *dirty = true;
                        edit_buffer.load_entity(world);
                    }
                }
                SelectedEntity::Burg(id) => {
                    ui.label(format!("Burg #{id}"));
                    rename_field(ui, "name", &mut edit_buffer.rename_buffer);
                    if ui.button("Apply").clicked() {
                        _ = vor_edit::rename_burg(world, id, &edit_buffer.rename_buffer);
                        *dirty = true;
                    }
                }
                SelectedEntity::Province(id) => {
                    ui.label(format!("Province #{id}"));
                    rename_field(ui, "name", &mut edit_buffer.rename_buffer);
                    if ui.button("Apply name").clicked() {
                        _ = vor_edit::rename_province(world, id, &edit_buffer.rename_buffer);
                        *dirty = true;
                    }
                    color_field(ui, "color", &mut edit_buffer.color_buffer);
                    if ui.button("Apply color").clicked() {
                        _ = vor_edit::set_province_color(world, id, &edit_buffer.color_buffer);
                        *dirty = true;
                        edit_buffer.load_entity(world);
                    }
                }
                SelectedEntity::Culture(_) => {}
            }
        } else {
            ui.label("(no editable entity)");
        }
    } else {
        ui.label("Right-click on map to inspect");
    }
}

/// Determines the most relevant entity for a cell.
fn entity_from_cell(world: &World, cell: usize) -> Option<SelectedEntity> {
    let sid = world.pack.cells.state.get(cell).copied().unwrap_or(0);
    if sid > 0 && world.states.iter().any(|s| s.id == sid && !s.removed) {
        return Some(SelectedEntity::State(sid));
    }
    let bid = world.pack.cells.burg.get(cell).copied().unwrap_or(0);
    if bid > 0 && world.burgs.iter().any(|b| b.id == bid && !b.removed) {
        return Some(SelectedEntity::Burg(bid));
    }
    let pid = world.pack.cells.province.get(cell).copied().unwrap_or(0);
    if pid > 0 && world.provinces.iter().any(|p| p.id == pid && !p.removed) {
        return Some(SelectedEntity::Province(pid));
    }
    None
}

fn rename_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.text_edit_singleline(value);
    });
}

fn color_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.text_edit_singleline(value);
    });
}
