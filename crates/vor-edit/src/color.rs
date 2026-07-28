//! Helpers de color hex (`#rrggbb`).
//!
//! Pequeño y enfocado: validar y normalizar. No depende de glam ni de wgpu —
//! eso vive en `vor-app` para el binding a egui.

use crate::error::EditError;

/// Valida que `hex` sea de la forma `#rrggbb` (6 dígitos hex después del `#`).
/// Reemplaza mayúsculas por minúsculas y re-completa el `#` si falta.
///
/// Retorna el string normalizado (siempre 7 chars, `#rrggbb` lowercase), o
/// `EditError::InvalidHexColor` si el formato no calza.
pub fn normalize_hex(hex: &str) -> Result<String, EditError> {
    let s = if let Some(rest) = hex.strip_prefix('#') {
        rest
    } else {
        hex
    };

    if s.len() != 6 || !s.is_ascii() || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EditError::InvalidHexColor(hex.to_string()));
    }

    Ok(format!("#{}", s.to_ascii_lowercase()))
}
