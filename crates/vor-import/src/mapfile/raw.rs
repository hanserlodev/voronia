//! Parser raw del `.map` de Azgaar — bytes → `RawMap { slots: Vec<String> }`.
//!
//! Pasos (replica `parseLoadedResult` de `azgaar-fmg/src/services/io/load.ts:167-197`):
//! 1. Leer bytes. Si no arranca con un `|` en los primeros 10 bytes, intentar gzip
//!    descompresión (`flate2::read::GzDecoder`).
//! 2. UTF-8 decode.
//! 3. SVG rescue: localizar `<svg id="map" ...</svg>` y reemplazar `\r\n` internos por `\n`
//!    (solo en ese bloque — el resto del archivo se splitea por `\r\n`).
//! 4. Split por `\r\n`.
//!
//! Notas bit-exactitud:
//! - El orden de los checks (gzip después de fail) replica Azgaar. En Rust lo hacemos
//!   "primero identificar header, fallback a gzip" — mismo resultado.
//! - El SVG rescue se hace solo si el bloque existe y contiene `\r\n`. Algunos `.map` muy
//!   antiguos pueden no tenerlo (entonces se omite el rescue).

use flate2::read::GzDecoder;
use std::io::Read;

/// Errores del parser raw.
#[derive(Debug, thiserror::Error)]
pub enum RawError {
    #[error("I/O error leyendo .map: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 inválido: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error(".map vacío o sin header válido (no se encontró `|` en slot 0)")]
    NoHeaderDelimiter,
}

/// Estructura raw tras el split. `slots[i]` corresponde al slot `[i]` del `.map` de Azgaar.
///
/// Algunos slots pueden estar vacíos (`""`) — sobretodo los deprecated y los opcionales
/// ausentes en mapas viejos. La longitud de `slots` es variable entre archivos.
#[derive(Debug, Clone)]
pub struct RawMap {
    /// Slots en orden `[0]`..`[N-1]`.
    pub slots: Vec<String>,
}

impl RawMap {
    /// Cantidad de slots en este `.map` raw.
    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// `true` si no hay slots.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Acceso por slot index. Retorna `None` si el índice está fuera de rango, o si
    /// el slot está como string vacío (Azgaar serializa slots opcionales ausentes como `""`).
    #[inline]
    pub fn get(&self, idx: usize) -> Option<&str> {
        self.slots
            .get(idx)
            .filter(|s| !s.is_empty())
            .map(String::as_str)
    }

    /// Acceso por índice, panicking si fuera de rango. Para uso interno del loader cuando
    /// el slot es mandatory.
    #[inline]
    pub fn must(&self, idx: usize) -> &str {
        self.slots
            .get(idx)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| panic!("slot [{idx}] esperado pero ausente o vacío"))
    }
}

/// Parsea bytes raw (contenido del archivo `.map`) en `RawMap`.
///
/// `bytes` pueden ser:
/// - texto plano UTF-8 con `\r\n` delimitadores y header `|`-delimited en slot `[0]`.
/// - bytes gzipped (si no arranca con `|` dentro de los primeros 10, probamos gzip).
pub fn parse(bytes: &[u8]) -> Result<RawMap, RawError> {
    // Quick header check: ¿luce como texto con `|` en los primeros 10 bytes?
    let looks_delimited = bytes.iter().take(10).any(|&b| b == b'|');

    let text: String = if looks_delimited {
        // UTF-8 decode directo (caso no comprimido).
        std::str::from_utf8(bytes)?.to_string()
    } else {
        // Intentar gzip descompresión. Si falla (no es gzip válido), error más informativo.
        let mut decoder = GzDecoder::new(bytes);
        let mut out = Vec::new();
        match decoder.read_to_end(&mut out) {
            Ok(_) => std::str::from_utf8(&out)?.to_string(),
            Err(_) => return Err(RawError::NoHeaderDelimiter),
        }
    };

    let content = rescue_svg_crlf(&text);
    let slots: Vec<String> = content.split("\r\n").map(|s| s.to_owned()).collect();

    if slots.is_empty() || !slots[0].contains('|') {
        return Err(RawError::NoHeaderDelimiter);
    }

    Ok(RawMap { slots })
}

/// Reemplaza `\r\n` por `\n` dentro del bloque `<svg id="map" ...</svg>` del contenido del .map.
///
/// Replica `load.ts:177-184`. Si el bloque no aparece, no se hace nada (caso de mapas
/// sin SVG serializado, que shouldn't occur pero defensa).
fn rescue_svg_crlf(content: &str) -> String {
    // Búsqueda simple del bloque `<svg` con `id="map"` ... `</svg>`. Sin usar regex para
    // mantener deps mínimas (la única regex en Cargo.toml sería añadir un crate).
    //
    // Estrategia: localizar `<svg` (primer match), buscar `id="map"` dentro, y luego
    // encontrar el `</svg>` matching. Como SVG bien-formado no permite nesting de `<svg>`
    // en este contexto (el `id="map"` es el outer), basta el primer `</svg>` después del start.
    let svg_start = match content.find("<svg") {
        Some(i) => i,
        None => return content.to_string(),
    };
    // Verificar `id="map"` dentro de los siguientes 200 chars del tag opening.
    let head = &content[svg_start..content.len().min(svg_start + 200)];
    if !head.contains("id=\"map\"") {
        return content.to_string();
    }
    // Buscar `</svg>` desde `svg_start`.
    let svg_end_rel = match content[svg_start..].find("</svg>") {
        Some(i) => i,
        None => return content.to_string(),
    };
    let svg_end = svg_start + svg_end_rel + "</svg>".len();

    let svg_block = &content[svg_start..svg_end];
    if !svg_block.contains("\r\n") {
        return content.to_string();
    }
    let corrected = svg_block.replace("\r\n", "\n");
    content.replace(svg_block, &corrected)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slot 0 simple con `|` delimitador, un slot Few y splits correctos.
    #[test]
    fn parse_text_simple() {
        let map = "1.0|seed|width|height\r\nslot1\r\nslot2\r\n";
        let raw = parse(map.as_bytes()).unwrap();
        assert_eq!(raw.len(), 4); // 3 slots + trailing empty (split por \r\n con trailing).
        assert_eq!(raw.must(0), "1.0|seed|width|height");
        assert_eq!(raw.must(1), "slot1");
        assert_eq!(raw.must(2), "slot2");
    }

    /// Slot vacío en el medio — `must` debe fallar, `get` retorna None.
    #[test]
    fn parse_handles_empty_slot() {
        let map = "1.0|seed\r\nfilled\r\n\r\nfilled_again\r\n";
        let raw = parse(map.as_bytes()).unwrap();
        assert_eq!(raw.must(0), "1.0|seed");
        assert_eq!(raw.must(1), "filled");
        assert!(raw.get(2).is_none(), "slot vacío");
        assert_eq!(raw.must(3), "filled_again");
    }

    /// Sin `|` en slot 0 → error (NoHeaderDelimiter).
    #[test]
    fn parse_no_header_delimiter_errors() {
        let map = "no_pipe_here\r\nslot1\r\n";
        let result = parse(map.as_bytes());
        assert!(matches!(result, Err(RawError::NoHeaderDelimiter)));
    }

    /// SVG CRLF rescue — el bloque `<svg id="map"...</svg>` con `\r\n` internos se
    /// reemplaza por `\n`. Los demás slots se mantienen splitteables.
    #[test]
    fn parse_svg_rescue_crlf() {
        let map = "1.0|seed\r\nslot1\r\n<svg id=\"map\">\r\n  <defs/>\r\n</svg>\r\nslot3\r\n";
        let raw = parse(map.as_bytes()).unwrap();
        // Tras rescue: el slot 2 (svg) tiene `\n` internos, no `\r\n`. Por eso el split
        // principal por `\r\n` lo deja intacto como un solo slot.
        assert_eq!(raw.get(2).unwrap(), "<svg id=\"map\">\n  <defs/>\n</svg>");
        assert_eq!(raw.must(3), "slot3");
    }

    /// gzip compressed — flate2 round trip.
    #[test]
    fn parse_gzip_compressed() {
        use std::io::Write;
        let original = "1.0|seed\r\nslot1\r\nslot2\r\n";
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        // compressed no arranca con `|`, así que el parser prueba gzip.
        let raw = parse(&compressed).unwrap();
        assert_eq!(raw.must(0), "1.0|seed");
        assert_eq!(raw.must(1), "slot1");
        assert_eq!(raw.must(2), "slot2");
    }
}
