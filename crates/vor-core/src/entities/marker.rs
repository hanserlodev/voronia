//! Marcador de punto de interés (slot `[35]`: `pack.markers` JSON).
//!
//! Ejemplo del Brample (fase-0 §12.3): `{"icon":"🌋","type":"volcanoes","dx":52,"px":13,"x":..,"y":..,"cell":..,"i":0}`.

/// Un marcador (pin de POI) sobre el mapa.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Marker {
    /// Id (su índice en `pack.markers`; el slot `[0]` de Azgaar suele ser `{}`).
    #[serde(default)]
    pub id: u32,
    /// Ícono (emoji o unicode): "🌋", "⚔️", etc.
    pub icon: String,
    /// Tipo de marcador: "volcanoes", "battlefields", "ruins",... (string mágico de Azgaar;
    /// Fase 1 lo preserva como `String` opaco; enum fuerte si Voronia lo normaliza más adelante).
    pub kind: String,
    /// Desplazamiento del label en `x` (estilo Azgaar).
    #[serde(default)]
    pub label_dx: i32,
    /// Tamaño del label en `px`.
    #[serde(default)]
    pub label_px: i32,
    /// Posición en el canvas `[x, y]`.
    #[serde(default)]
    pub position: [f32; 2],
    /// Celda del pack sobre la que cae (para vínculo geográfico).
    #[serde(default)]
    pub cell: u32,
    /// Texto legend/legendario asociado (opcional).
    #[serde(default)]
    pub legend: Option<String>,
    /// Nota libre del usuario (id → `Note` en slot `[4]`).
    #[serde(default)]
    pub note_id: Option<u32>,
    /// `true` si el marker está "oculto" / removido manualmente en la UI.
    #[serde(default)]
    pub removed: bool,
}
