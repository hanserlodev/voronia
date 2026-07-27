//! `World` — raíz del World Data Model, lo que `vor-import` produce poblando todo lo demás.

use crate::coordinates::MapCoordinates;
use crate::entities::{
    biome::Biome, burg::Burg, culture::Culture, ice::Ice, marker::Marker, measurer::Measurer,
    namebase::NameBase, note::Note, province::Province, religion::Religion, river::River,
    route::Route, state::State, zone::Zone,
};
use crate::grid::Grid;
use crate::pack::Pack;
use crate::settings::{MapHeader, Settings};

/// El World Data Model completo: un mapa ya cargado o generado.
///
/// `vor-import` pobla este struct regenerando geometría (Grid/Pack con cell counts y
/// mallas Voronoi) y aplicando atributos parseados desde el `.map`. Todo layout aquí
/// **es SoA** para atributos por celda (plan §7.2; conventions.md §"Layout de datos").
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct World {
    /// Header del `.map` (slot `[0]`).
    pub header: MapHeader,
    /// Settings del `.map` (slot `[1]`).
    pub settings: Settings,
    /// Coordenadas geográficas del mapa (slot `[2]`).
    pub coordinates: MapCoordinates,

    /// Malla del grid (regenerada desde semilla; atribuidos slots `[7]`–`[11]`).
    pub grid: Grid,
    /// Malla del pack (regenerada via `reGraph`; atribuidos slots `[16]`–`[44]`).
    pub pack: Pack,

    // --- Catálogos ---
    /// Catálogo de biomas (slot `[3]`_pipe-delimited).
    pub biomes: Vec<Biome>,
    /// Notas/leyendas (slot `[4]` JSON).
    pub notes: Vec<Note>,
    /// Cultura (`pack.cultures`, slot `[13]`).
    pub cultures: Vec<Culture>,
    /// Estados (`pack.states`, slot `[14]`).
    pub states: Vec<State>,
    /// Burgos (`pack.burgs`, slot `[15]`).
    pub burgs: Vec<Burg>,
    /// Religiones (`pack.religions`, slot `[29]`).
    pub religions: Vec<Religion>,
    /// Provincias (`pack.provinces`, slot `[30]`).
    pub provinces: Vec<Province>,
    /// Ríos (`pack.rivers`, slot `[32]`).
    pub rivers: Vec<River>,
    /// Markers (`pack.markers`, slot `[35]`).
    pub markers: Vec<Marker>,
    /// Rutas (`pack.routes`, slot `[37]`).
    pub routes: Vec<Route>,
    /// Zonas custom (`pack.zones`, slot `[38]`).
    pub zones: Vec<Zone>,
    /// Hielo (`pack.ice`, slot `[39]`).
    pub ice: Vec<Ice>,
    /// NameBases (`namesData`, slot `[31]`).
    pub namebases: Vec<NameBase>,
    /// Reglas de medición (`pack.measurers`, slot `[46]`).
    pub measurers: Vec<Measurer>,

    // --- Economía y milicia (Fase 7) ---
    //
    // Los slots `[40]`–`[44]` (goods/markets/deals) y el subtree de `military`
    // dentro de `State::military` son propios del subsistema de economía y milicia
    // de Azgaar, que Voronia implementa en Fase 7. Para Fase 1 los preservamos opacos
    // (re-export sin pérdidas) dentro de `State` o como `serde_json::Value`.
    /// Fonts (slot `[34]`) — Ajgaar guarda config de tipografías; preservar opaco.
    /// No tiene efecto en `vor-render` (Voronia usa sus propias font stacks).
    #[serde(default, with = "crate::serde_json_string")]
    pub fonts: serde_json::Value,
    /// Custom good icons (slot `[45]`) — HTML outer string; preservar opaco.
    #[serde(default, with = "crate::serde_json_string")]
    pub custom_good_icons: serde_json::Value,
    /// Economía: `pack.goods`, `pack.markets`, `pack.deals` (slots `[41]`/`[42]`/`[43]`).
    /// Preservamos como `serde_json::Value` para no modelar el subsistema en Fase 1.
    #[serde(default, with = "crate::serde_json_string")]
    pub goods: serde_json::Value,
    #[serde(default, with = "crate::serde_json_string")]
    pub markets: serde_json::Value,
    #[serde(default, with = "crate::serde_json_string")]
    pub deals: serde_json::Value,
}
