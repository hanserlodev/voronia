use std::io::Read;
use std::path::Path;

use vor_core::world::World;

use crate::error::FormatError;
use crate::header::{VornHeader, VornMetadata};

/// Loads a `World` from a `.vorn` file.
///
/// Reads the header, validates magic + version, then deserializes metadata and payload.
pub fn load(path: impl AsRef<Path>) -> Result<(World, VornMetadata), FormatError> {
    let mut file = std::fs::File::open(path.as_ref())?;

    // Read fixed 16-byte header
    let mut header_buf = [0u8; 16];
    file.read_exact(&mut header_buf)?;
    let header = VornHeader::decode(&header_buf)?;

    // Read metadata
    let mut meta_buf = vec![0u8; header.metadata_len as usize];
    file.read_exact(&mut meta_buf)?;
    let metadata: VornMetadata = bincode::deserialize(&meta_buf)?;

    // Read payload (rest of the file)
    let mut payload = Vec::new();
    file.read_to_end(&mut payload)?;

    let world: World = bincode::deserialize(&payload)?;
    Ok((world, metadata))
}

/// Loads only the metadata of a `.vorn` (without deserializing the full World).
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
