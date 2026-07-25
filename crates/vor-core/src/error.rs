//! Errores del World Data Model de Voronia.
//!
//! `vor-core` es puro: no parsea archivos ni valida IDs contra geometría —
//! eso es responsabilidad de `vor-import`. Acá solo viven errores que pueden
//! surgir al construir o usar los tipos puros (IDs fuera de rango, invariants
//! de layout SoA, enums desconocidos al deserializar).

use thiserror::Error;

/// Error de `vor-core`. Tipos de la librería usan `thiserror` (regla de `references/conventions.md` §"Manejo de errores").
#[derive(Debug, Error)]
pub enum CoreError {
    /// Un índice de celda/feature/entidad cae fuera del rango válido de su colección.
    #[error("índice fuera de rango: {what} {index} no existe en una colección de largo {len}")]
    OutOfRange {
        what: &'static str,
        index: usize,
        len: usize,
    },

    /// Invariante de layout Structure-of-Arrays roto: dos `Vec` que deben indexarse
    /// en paralelo por id de celda tienen largos distintos.
    #[error("layout SoA inconsistente: el campo `{field}` tiene largo {actual} pero se esperaba {expected} (igual al resto de celdas)")]
    SoaLengthMismatch {
        field: &'static str,
        expected: usize,
        actual: usize,
    },

    /// Variant de enum desconocida al deserializar (ej. `FeatureType` con un string
    /// que no calza con ningún variant conocido). El `raw` preserva el valor original
    /// para diagnóstico y para permitir re-exportarlo sin pérdida.
    #[error("variant de enum desconocida para {enum_name}: {raw:?}")]
    UnknownEnumVariant {
        enum_name: &'static str,
        raw: String,
    },
}
