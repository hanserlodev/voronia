//! Raw parser of the Azgaar `.map` — bytes → `RawMap { slots: Vec<String> }`.
//!
//! Steps (replicates `parseLoadedResult` of `azgaar-fmg/src/services/io/load.ts:167-197`):
//! 1. Read bytes. If it does not start with a `|` in the first 10 bytes, try gzip
//!    decompression (`flate2::read::GzDecoder`).
//! 2. UTF-8 decode.
//! 3. SVG rescue: locate `<svg id="map" ...</svg>` and replace internal `\r\n` with `\n`
//!    (only in that block — the rest of the file is split by `\r\n`).
//! 4. Split by `\r\n`.
//!
//! Bit-exactness notes:
//! - The order of checks (gzip after fail) replicates Azgaar. In Rust we do
//!   "identify header first, fall back to gzip" — same result.
//! - The SVG rescue is done only if the block exists and contains `\r\n`. Some very
//!   old `.map` files may not have it (then the rescue is skipped).

use flate2::read::GzDecoder;
use std::io::Read;

/// Errors of the raw parser.
#[derive(Debug, thiserror::Error)]
pub enum RawError {
    #[error("I/O error reading .map: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error(".map empty or without a valid header (no `|` found in slot 0)")]
    NoHeaderDelimiter,
}

/// Raw structure after the split. `slots[i]` corresponds to slot `[i]` of the Azgaar `.map`.
///
/// Some slots may be empty (`""`) — especially deprecated and optional slots
/// absent in old maps. The length of `slots` varies between files.
#[derive(Debug, Clone)]
pub struct RawMap {
    /// Slots in order `[0]`..`[N-1]`.
    pub slots: Vec<String>,
}

impl RawMap {
    /// Number of slots in this raw `.map`.
    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// `true` if there are no slots.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Access by slot index. Returns `None` if the index is out of range, or if
    /// the slot is an empty string (Azgaar serializes absent optional slots as `""`).
    #[inline]
    pub fn get(&self, idx: usize) -> Option<&str> {
        self.slots
            .get(idx)
            .filter(|s| !s.is_empty())
            .map(String::as_str)
    }

    /// Access by index, panicking if out of range. For internal loader use when
    /// the slot is mandatory.
    #[inline]
    pub fn must(&self, idx: usize) -> &str {
        self.slots
            .get(idx)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| panic!("slot [{idx}] expected but absent or empty"))
    }
}

/// Parses raw bytes (the contents of a `.map` file) into `RawMap`.
///
/// `bytes` can be:
/// - plain UTF-8 text with `\r\n` delimiters and a `|`-delimited header in slot `[0]`.
/// - gzipped bytes (if it does not start with `|` within the first 10, we try gzip).
pub fn parse(bytes: &[u8]) -> Result<RawMap, RawError> {
    // Quick header check: does it look like text with `|` in the first 10 bytes?
    let looks_delimited = bytes.iter().take(10).any(|&b| b == b'|');

    let text: String = if looks_delimited {
        // Direct UTF-8 decode (uncompressed case).
        std::str::from_utf8(bytes)?.to_string()
    } else {
        // Try gzip decompression. If it fails (not valid gzip), a more informative error.
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

/// Replaces `\r\n` with `\n` inside the `<svg id="map" ...</svg>` block of the .map content.
///
/// Replicates `load.ts:177-184`. If the block does not appear, nothing is done (case of
/// maps without serialized SVG, which should not occur but is defensive).
fn rescue_svg_crlf(content: &str) -> String {
    // Simple search for the `<svg` block with `id="map"` ... `</svg>`. Without regex to
    // keep dependencies minimal (the only regex in Cargo.toml would add a crate).
    //
    // Strategy: locate `<svg` (first match), look for `id="map"` inside, then find the
    // matching `</svg>`. Since well-formed SVG does not allow `<svg>` nesting
    // in this context (the `id="map"` is the outer one), the first `</svg>` after the start suffices.
    let svg_start = match content.find("<svg") {
        Some(i) => i,
        None => return content.to_string(),
    };
    // Verify `id="map"` within the next 200 chars of the opening tag.
    let head = &content[svg_start..content.len().min(svg_start + 200)];
    if !head.contains("id=\"map\"") {
        return content.to_string();
    }
    // Find `</svg>` starting from `svg_start`.
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

    /// Simple slot 0 with `|` delimiter, a few slots and correct splits.
    #[test]
    fn parse_text_simple() {
        let map = "1.0|seed|width|height\r\nslot1\r\nslot2\r\n";
        let raw = parse(map.as_bytes()).unwrap();
        assert_eq!(raw.len(), 4); // 3 slots + trailing empty (split by \r\n with trailing).
        assert_eq!(raw.must(0), "1.0|seed|width|height");
        assert_eq!(raw.must(1), "slot1");
        assert_eq!(raw.must(2), "slot2");
    }

    /// Empty slot in the middle — `must` must fail, `get` returns None.
    #[test]
    fn parse_handles_empty_slot() {
        let map = "1.0|seed\r\nfilled\r\n\r\nfilled_again\r\n";
        let raw = parse(map.as_bytes()).unwrap();
        assert_eq!(raw.must(0), "1.0|seed");
        assert_eq!(raw.must(1), "filled");
        assert!(raw.get(2).is_none(), "empty slot");
        assert_eq!(raw.must(3), "filled_again");
    }

    /// No `|` in slot 0 → error (NoHeaderDelimiter).
    #[test]
    fn parse_no_header_delimiter_errors() {
        let map = "no_pipe_here\r\nslot1\r\n";
        let result = parse(map.as_bytes());
        assert!(matches!(result, Err(RawError::NoHeaderDelimiter)));
    }

    /// SVG CRLF rescue — the `<svg id="map"...</svg>` block with internal `\r\n` is
    /// replaced by `\n`. The other slots remain splittable.
    #[test]
    fn parse_svg_rescue_crlf() {
        let map = "1.0|seed\r\nslot1\r\n<svg id=\"map\">\r\n  <defs/>\r\n</svg>\r\nslot3\r\n";
        let raw = parse(map.as_bytes()).unwrap();
        // After rescue: slot 2 (svg) has internal `\n`, not `\r\n`. So the main
        // split by `\r\n` keeps it intact as a single slot.
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
        // compressed does not start with `|`, so the parser tries gzip.
        let raw = parse(&compressed).unwrap();
        assert_eq!(raw.must(0), "1.0|seed");
        assert_eq!(raw.must(1), "slot1");
        assert_eq!(raw.must(2), "slot2");
    }
}
