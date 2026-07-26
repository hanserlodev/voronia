//! Parser del formato binario-texto `.map` de Azgaar Fantasy Map Generator.
//!
//! Formato: array de strings unidos por `\r\n`. Cada slot es un string.
//! Slots indexados `[0]`..`[45]` aprox. (see `docs/fase-0-investigacion.md` §12
//! para la disección del "Brample").
//!
//! Compresión gzip **opcional**: si el archivo no arranca con un slot `[0]`
//! conteniendo `|` (header delimited de Azgaar), se reintenta descomprimiendo gzip.
//! Esto replica `parseLoadedResult` (`azgaar-fmg/src/services/io/load.ts:167-197`).
//!
//! SVG rescue: el bloque `<svg id="map"...</svg>` (slot `[5]` típicamente) puede
//! tener CRLF internos, lo que rompería el split. Antes de splitear, se localiza el
//! bloque y se reemplazan `\r\n` internos por `\n` en él (`load.ts:177-184`).
//!
//! El parser está estratificado:
//! - `RawMap`: slots en string crudo (post-split, post-SVG-rescue, post-decompress-gzip).
//! - `Header`: parseo del slot `[0]` delimitado por `|`.
//! - `Settings`: parseo del slot `[1]` delimitado por `|`.
//! - `Loader::load(raw) -> World`: pobla un `vor_core::World` con slots mapeados a
//!   tipos fuertes + geometría regenerada desde la seed del header.

pub mod catalogs;
pub mod cells;
pub mod header;
pub mod loader;
pub mod raw;

pub use loader::{LoadError, LoadResult, Loader};
pub use raw::RawMap;
