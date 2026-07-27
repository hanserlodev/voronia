use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("magic bytes inválidos: esperados VORN, encontrados {found:?}")]
    InvalidMagic { found: [u8; 4] },

    #[error("versión de formato no soportada: {0}")]
    UnsupportedVersion(u16),

    #[error("checksum mismatch: esperado {expected}, calculado {actual}")]
    ChecksumMismatch { expected: u32, actual: u32 },
}
