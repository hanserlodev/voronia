//! Mutators for `Province` entities.

use vor_core::world::World;

use crate::error::EditError;

fn find_province_mut(
    world: &mut World,
    id: u16,
) -> Result<&mut vor_core::entities::province::Province, EditError> {
    if id == 0 {
        return Err(EditError::EntityNotFound {
            what: "province",
            id: 0,
            len: world.provinces.len(),
        });
    }
    let len = world.provinces.len();
    world
        .provinces
        .iter_mut()
        .find(|p| p.id == id && !p.removed)
        .ok_or(EditError::EntityNotFound {
            what: "province",
            id,
            len,
        })
}

pub fn rename_province(
    world: &mut World,
    province_id: u16,
    new_name: &str,
) -> Result<(), EditError> {
    let name = new_name.trim();
    if name.is_empty() {
        return Err(EditError::EmptyName {
            what: "province",
            id: province_id,
        });
    }
    find_province_mut(world, province_id)?.name = name.to_string();
    Ok(())
}

pub fn set_province_color(world: &mut World, province_id: u16, hex: &str) -> Result<(), EditError> {
    let color = crate::color::normalize_hex(hex)?;
    find_province_mut(world, province_id)?.color = color;
    Ok(())
}
