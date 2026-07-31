//! Hex color helpers (`#rrggbb`).
//!
//! Small and focused: validate and normalize. Does not depend on glam or wgpu —
//! that lives in `vor-app` for the egui binding.

use crate::error::EditError;

/// Validates that `hex` has the form `#rrggbb` (6 hex digits after the `#`).
/// Lowercases uppercase letters and re-adds the `#` if missing.
///
/// Returns the normalized string (always 7 chars, `#rrggbb` lowercase), or
/// `EditError::InvalidHexColor` if the format does not match.
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
