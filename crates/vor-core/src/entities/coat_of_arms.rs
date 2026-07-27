//! Blasón / escudo de armas. Formato compatible con "Armoria" de Watabou (MFCG/MFCG-Origen),
//! para que Voronia pueda importar/exportar el mismo formato que Azgaar usa hoy.

/// Blasón de una entidad (burgo, estado). El formato exacto de los campos se deja
/// como `serde_json::Value` hasta que se valide contra la wiki de Armoria en Fase 0/1
/// (es un sub-formato no documentado a fondo en el plan maestro).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CoatOfArms {
    /// Payload opaco del blasón. Preserva todo el JSON de Azgaar (= interoperabilidad
    /// total con Armoria/MFCG sin perder datos al importar).
    #[serde(default, with = "crate::serde_json_string")]
    pub payload: serde_json::Value,
}
