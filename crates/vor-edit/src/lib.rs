//! `vor-edit` — Comandos de edición y undo/redo de Voronia.
//!
//! En Fase 5 los comandos son mutación directa (sin Command stack). El caller
//! marca la app como dirty y regenera meshes de capas afectadas. El undo/redo
//! con patrón Command llega en Fase 6.
//!
//! Los mutadores siempre buscan entidades por campo `id` (no por posición en el
//! Vec), porque el loader de `.map` omite placeholders con `skip(1)`.

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

/// Estado de edición efímero para bindings de egui.
///
/// Las ediciones ocurren en el World real a través de las funciones de este
/// crate; este struct solo trackea dirty flag y buffers de strings temporales
/// para que egui pueda mostrar campos de texto sin pedirle al usuario que
/// "confirme" cada pulsación de tecla.
#[derive(Debug, Clone, Default)]
pub struct EditBuffer {
    pub selected_entity_id: Option<SelectedEntity>,
    /// String temporal para el campo de rename (egui text edit).
    pub rename_buffer: String,
    /// String temporal para el campo de color hex.
    pub color_buffer: String,
    /// Flag dirty: se marca `true` tras cada mutación. La app lo lee y lo
    /// pone a `false` después de regenerar meshes.
    pub dirty: bool,
}

/// Qué entidad está seleccionada en el inspector.
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

    /// Puebla los buffers desde la entidad seleccionada (para bindear a egui).
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
                // No editable en Fase 5
            }
            None => {}
        }
    }
}
