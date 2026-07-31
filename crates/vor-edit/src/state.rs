//! Mutators for `State` entities.
//!
//! Mutate fields of the state catalog in `World`. Mutation is direct
//! (no Command stack; that arrives in Phase 6). The caller is responsible for
//! marking the app as dirty and regenerating meshes of affected layers.
//!
//! States are looked up by `id` (the `State::id` field) and not by position
//! in the Vec, because the `.map` loader does `skip(1)` without inserting a
//! placeholder at position 0. Using `find` is correct regardless of layout.

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
