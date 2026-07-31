//! Mutators for `Burg` entities.

use vor_core::world::World;

use crate::error::EditError;

fn find_burg_mut(
    world: &mut World,
    id: u16,
) -> Result<&mut vor_core::entities::burg::Burg, EditError> {
    let len = world.burgs.len();
    world
        .burgs
        .iter_mut()
        .find(|b| b.id == id && b.id != 0 && !b.removed)
        .ok_or(EditError::EntityNotFound {
            what: "burg",
            id,
            len,
        })
}

pub fn rename_burg(world: &mut World, burg_id: u16, new_name: &str) -> Result<(), EditError> {
    let name = new_name.trim();
    if name.is_empty() {
        return Err(EditError::EmptyName {
            what: "burg",
            id: burg_id,
        });
    }
    find_burg_mut(world, burg_id)?.name = name.to_string();
    Ok(())
}

pub fn set_burg_population(
    world: &mut World,
    burg_id: u16,
    new_population: f32,
) -> Result<(), EditError> {
    let cell = {
        let b = find_burg_mut(world, burg_id)?;
        b.population = new_population.max(0.0);
        b.cell
    };
    if let Some(slot) = world.pack.cells.population.get_mut(cell as usize) {
        *slot = new_population.max(0.0);
    }
    Ok(())
}

pub fn toggle_burg_capital(
    world: &mut World,
    burg_id: u16,
    is_capital: bool,
) -> Result<(), EditError> {
    let (state_id, cell_id) = {
        let b = find_burg_mut(world, burg_id)?;
        b.is_capital = is_capital;
        (b.state, b.cell)
    };

    if is_capital && state_id > 0 {
        for b in world.burgs.iter_mut() {
            if b.id != burg_id && b.state == state_id {
                b.is_capital = false;
            }
        }
        if let Some(s) = world.states.iter_mut().find(|s| s.id == state_id) {
            s.center_cell = cell_id;
        }
    }
    Ok(())
}
