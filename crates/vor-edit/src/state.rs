//! Mutadores de entidades `State`.
//!
//! Mutan campos del catálogo de estados en `World`. La mutación es directa
//! (sin Command stack; eso llega en Fase 6). El caller es responsable de
//! marcar la app como dirty y de regenerar meshes de capas afectadas.
//!
//! Los estados se buscan por `id` (campo `State::id`) y no por posición en el
//! Vec, porque el loader de `.map` hace `skip(1)` sin insertar placeholder en
//! posición 0. Usar `find` es correcto independientemente del layout.

use vor_core::entities::state::GovernmentForm;
use vor_core::world::World;

use crate::error::EditError;

fn find_state_mut(
    world: &mut World,
    id: u16,
) -> Result<&mut vor_core::entities::state::State, EditError> {
    if id == 0 {
        return Err(EditError::EntityNotFound {
            what: "state",
            id: 0,
            len: world.states.len(),
        });
    }
    let len = world.states.len();
    world
        .states
        .iter_mut()
        .find(|s| s.id == id && !s.removed)
        .ok_or(EditError::EntityNotFound {
            what: "state",
            id,
            len,
        })
}

pub fn rename_state(world: &mut World, state_id: u16, new_name: &str) -> Result<(), EditError> {
    let name = new_name.trim();
    if name.is_empty() {
        return Err(EditError::EmptyName {
            what: "state",
            id: state_id,
        });
    }
    find_state_mut(world, state_id)?.name = name.to_string();
    Ok(())
}

pub fn set_state_color(world: &mut World, state_id: u16, hex: &str) -> Result<(), EditError> {
    let color = crate::color::normalize_hex(hex)?;
    find_state_mut(world, state_id)?.color = color;
    Ok(())
}

pub fn set_state_form(
    world: &mut World,
    state_id: u16,
    form: GovernmentForm,
) -> Result<(), EditError> {
    find_state_mut(world, state_id)?.form = form;
    Ok(())
}
