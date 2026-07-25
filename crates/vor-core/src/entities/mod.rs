//! Entidades del mundo: culturas, estados, burgos, religiones, provincias, ríos,
//! marcadores, rutas, zonas, hielo. Listado consolidado para no llenar `lib.rs`.
//!
//! Cada entidad aquí sigue el modelo de datos confirmado contra Azgaar (refs en
//! `voronia-plan-proyecto.md` §7.4–§7.7 y `docs/fase-0-investigacion.md` §10.1).
//! Los enums del modelo de Azgaar con variants que requieren confirmación contra
//! la wiki están marcados `// TODO Fase 0/1: confirmar variants contra wiki` — aun
//! así los dejamos cerrados para no perder tipado fuerte.

pub mod biome;
pub mod burg;
pub mod coat_of_arms;
pub mod culture;
pub mod ice;
pub mod marker;
pub mod measurer;
pub mod namebase;
pub mod note;
pub mod province;
pub mod religion;
pub mod river;
pub mod route;
pub mod state;
pub mod zone;
