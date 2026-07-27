use std::io::Read;
use std::path::Path;

use vor_core::world::World;

use crate::error::FormatError;
use crate::header::{VornHeader, VornMetadata};

/// Carga un `World` desde un archivo `.vorn`.
///
/// Lee el header, valida magic + versión, luego deserializa metadata y payload.
pub fn load(path: impl AsRef<Path>) -> Result<(World, VornMetadata), FormatError> {
    let mut file = std::fs::File::open(path.as_ref())?;

    // Leer header fijo de 16 bytes
    let mut header_buf = [0u8; 16];
    file.read_exact(&mut header_buf)?;
    let header = VornHeader::decode(&header_buf)?;

    // Leer metadata
    let mut meta_buf = vec![0u8; header.metadata_len as usize];
    file.read_exact(&mut meta_buf)?;
    let metadata: VornMetadata = bincode::deserialize(&meta_buf)?;

    // Leer payload (resto del archivo)
    let mut payload = Vec::new();
    file.read_to_end(&mut payload)?;

    let world: World = bincode::deserialize(&payload)?;
    Ok((world, metadata))
}

/// Carga solo el metadata de un `.vorn` (sin deserializar el World completo).
pub fn load_metadata(path: impl AsRef<Path>) -> Result<VornMetadata, FormatError> {
    let mut file = std::fs::File::open(path.as_ref())?;

    let mut header_buf = [0u8; 16];
    file.read_exact(&mut header_buf)?;
    let header = VornHeader::decode(&header_buf)?;

    let mut meta_buf = vec![0u8; header.metadata_len as usize];
    file.read_exact(&mut meta_buf)?;
    let metadata: VornMetadata = bincode::deserialize(&meta_buf)?;
    Ok(metadata)
}
