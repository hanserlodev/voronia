//! Note (slot `[4]`: notes JSON). Texto libre del usuario asociado a cualquier entidad
//! (burgo, estado, marker, etc.). Relevante para la integración con Atenea (plan §22).

/// Una nota de leyenda/texto libre.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Note {
    pub id: u32,
    /// Texto de la leyenda (puede ser multi-línea).
    #[serde(default)]
    pub content: String,
    /// Id de la entidad vinculada (burgo/state/marker...). El tipo se infiere por contexto
    /// en Azgaar; lo preservamos como `u32` opaco por ahora.
    #[serde(default)]
    pub linked_id: Option<u32>,
}
