use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("invalid magic bytes: expected VORN, found {found:?}")]
    InvalidMagic { found: [u8; 4] },

    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u16),

    #[error("checksum mismatch: expected {expected}, computed {actual}")]
    ChecksumMismatch { expected: u32, actual: u32 },
}
