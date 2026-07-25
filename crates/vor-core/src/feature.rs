//! Features: continentes, islas, lagos y océanos.
//!
//! Cada celda del grid/pack pertenece a exactamente una feature (`cell.feature_id`).
//! En Azgaar, las features se numeran con `0` como slot reservado/no-land (el slot
//! `[0]` de `pack.features` es un placeholder vacío). En Voronia traducimos el
//! modelo a tipos fuertes: nada de strings mágicos para `kind`.

/// Gran grupo de una feature de tierra (cumple `height >= 20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LandGroup {
    Continent,
    Island,
    Isle,
    LakeIsland,
}

/// Gran grupo de una feature de agua (lago). El océano no necesita subgrupo aquí —
/// se distingue por `Feature::touches_border`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LakeGroup {
    Freshwater,
    Salt,
    Dry,
    Sinkhole,
    Lava,
}

/// Tipo de feature. Refleja el árbol decisión de Azgaar:
/// `(is_land)? tierra : (touches_border)? océano : lago`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FeatureType {
    /// Cuerpo de agua abierto al borde del canvas.
    #[default]
    Ocean,
    /// Masa de tierra — la primera feature de tierra se considera `Continent`,
    /// las siguientes `Island`/`Isle` según tamaño (umbral a confirmar contra wiki en Fase 0).
    Island,
    /// Cuerpo de agua cerrado (no toca borde) — un lago. Subgrupo via `LakeGroup`.
    Lake,
}

/// Una feature (isla/lago/océano) identificada por `id`.
///
/// Equivale a los items de `pack.features` (slot `[12]` del `.map`).
/// En Azgaar, el item `[0]` es un placeholder reservado; en Voronia lo
/// mantenemos como `Feature::placeholder()` opcional, o se omite y se indexa desde 1.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Feature {
    /// Id de la feature (1-based tras descartar el slot 0.
    pub id: u32,
    /// `true` si la feature es tierra (`height >= 20` en todas sus celdas).
    pub is_land: bool,
    /// `true` si la feature toca el borde del canvas — distingue lago de océano.
    pub touches_border: bool,
    /// Tipo de la feature.
    pub kind: FeatureType,
    /// Subgrupo de tierra (`Some` solo si `is_land`).
    pub land_group: Option<LandGroup>,
    /// Subgrupo de lago (`Some` solo si `kind == Lake`).
    pub lake_group: Option<LakeGroup>,
    /// Cantidad de celdas que conforman la feature.
    pub cell_count: u32,
    /// Id de la primera celda de la feature (representante).
    pub first_cell: u32,
    /// Vértices del perímetro (puntos en sentido horario/anti-horario; confirmar contra código real en Fase 1).
    pub perimeter_vertices: Vec<u32>,
    /// Nombre del lago, si tiene (las features de tierra/océano no llevan nombre acá).
    pub name: Option<String>,
}

impl Feature {
    /// Placeholder reservado — refleja el slot `[0]` que deja Azgaar en `pack.features`.
    /// Útil para mantener mapeo id→index literal si se replica el layout de Azgaar.
    #[inline]
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            is_land: false,
            touches_border: false,
            kind: FeatureType::Ocean,
            land_group: None,
            lake_group: None,
            cell_count: 0,
            first_cell: 0,
            perimeter_vertices: Vec::new(),
            name: None,
        }
    }
}
