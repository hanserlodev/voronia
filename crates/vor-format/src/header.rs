use serde::{Deserialize, Serialize};

pub const VORN_MAGIC: [u8; 4] = *b"VORN";
pub const VORN_FORMAT_VERSION: u16 = 1;

/// Fixed-size header del Vorn World File.
///
/// Layout en disco (16 bytes):
///   [0..4]  magic: b"VORN"
///   [4..6]  format_version: u16 LE
///   [6..10] metadata_len: u32 LE (bytes del VornMetadata serializado)
///   [10]    compression: u8 (0 = none, 1 = gzip — reservado para futuro)
///   [11..16] _reserved: [u8; 5]
///
/// Sigue el metadata (bincode), luego el payload (bincode World).
#[derive(Debug, Clone)]
pub struct VornHeader {
    pub format_version: u16,
    pub metadata_len: u32,
    pub compression: u8,
}

impl VornHeader {
    pub fn new(metadata_len: u32) -> Self {
        Self {
            format_version: VORN_FORMAT_VERSION,
            metadata_len,
            compression: 0,
        }
    }

    pub fn encode(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&VORN_MAGIC);
        buf[4..6].copy_from_slice(&self.format_version.to_le_bytes());
        buf[6..10].copy_from_slice(&self.metadata_len.to_le_bytes());
        buf[10] = self.compression;
        buf
    }

    pub fn decode(bytes: &[u8; 16]) -> Result<Self, super::error::FormatError> {
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != VORN_MAGIC {
            return Err(super::error::FormatError::InvalidMagic { found: magic });
        }
        let format_version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        if format_version != VORN_FORMAT_VERSION {
            return Err(super::error::FormatError::UnsupportedVersion(
                format_version,
            ));
        }
        let metadata_len = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
        let compression = bytes[10];
        Ok(Self {
            format_version,
            metadata_len,
            compression,
        })
    }
}

/// Metadata del mapa guardado en el Vorn Header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VornMetadata {
    /// Nombre del mapa.
    pub map_name: String,
    /// Semilla procedural.
    pub seed: String,
    /// Fecha de creación/exportación.
    pub date: String,
    /// Versión de Voronia que generó/exportó el archivo.
    pub voronia_version: String,
    /// Versión de Azgaar de la que se importó (si aplica).
    pub azgaar_version: Option<String>,
}

impl VornMetadata {
    pub fn new(
        map_name: impl Into<String>,
        seed: impl Into<String>,
        date: impl Into<String>,
        voronia_version: impl Into<String>,
        azgaar_version: Option<impl Into<String>>,
    ) -> Self {
        Self {
            map_name: map_name.into(),
            seed: seed.into(),
            date: date.into(),
            voronia_version: voronia_version.into(),
            azgaar_version: azgaar_version.map(|s| s.into()),
        }
    }
}
