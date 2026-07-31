//! `vor-edit` — Voronia editing commands and undo/redo.
//!
//! In Phase 5 commands are direct mutation (no Command stack). The caller
//! marks the app as dirty and regenerates meshes of affected layers. Undo/redo
//! with the Command pattern arrives in Phase 6.
//!
//! Mutators always look up entities by `id` field (not by position in the
//! Vec), because the `.map` loader omits placeholders with `skip(1)`.

pub mod burg;
pub mod color;
pub mod error;
pub mod province;
pub mod state;

pub use burg::{rename_burg, set_burg_population, toggle_burg_capital};
pub use color::normalize_hex;
pub use error::EditError;
pub use province::{rename_province, set_province_color};
pub use state::{rename_state, set_state_color, set_state_form};

/// Ephemeral editing state for egui bindings.
///
/// Edits happen on the real World through this crate's functions; this struct
/// only tracks the dirty flag and temporary string buffers so egui can show
/// text fields without asking the user to "confirm" each keystroke.
#[derive(Debug, Clone, Default)]
pub struct EditBuffer {
    pub selected_entity_id: Option<SelectedEntity>,
    /// Temporary string for the rename field (egui text edit).
    pub rename_buffer: String,
    /// Temporary string for the hex color field.
    pub color_buffer: String,
    /// Dirty flag: set to `true` after each mutation. The app reads it and
    /// resets it to `false` after regenerating meshes.
    pub dirty: bool,
}

/// Which entity is selected in the inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedEntity {
    State(u16),
    Burg(u16),
    Province(u16),
    Culture(u16),
}

impl EditBuffer {
    pub fn clear_selection(&mut self) {
        self.selected_entity_id = None;
        self.rename_buffer.clear();
        self.color_buffer.clear();
    }

    /// Populates the buffers from the selected entity (to bind to egui).
    pub fn load_entity(&mut self, world: &vor_core::World) {
        match self.selected_entity_id {
            Some(SelectedEntity::State(id)) => {
                if let Some(s) = world.states.iter().find(|s| s.id == id) {
                    self.rename_buffer = s.name.clone();
                    self.color_buffer = s.color.clone();
                }
            }
            Some(SelectedEntity::Province(id)) => {
                if let Some(p) = world.provinces.iter().find(|p| p.id == id) {
                    self.rename_buffer = p.name.clone();
                    self.color_buffer = p.color.clone();
                }
            }
            Some(SelectedEntity::Burg(id)) => {
                if let Some(b) = world.burgs.iter().find(|b| b.id == id) {
                    self.rename_buffer = b.name.clone();
                }
            }
            Some(SelectedEntity::Culture(_id)) => {
                // Not editable in Phase 5
            }
            None => {}
        }
    }
}
