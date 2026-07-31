use std::io::Write;
use std::path::Path;

use vor_core::world::World;

use crate::error::FormatError;
use crate::header::{VornHeader, VornMetadata};

/// Saves a `World` in `.vorn` format.
///
/// Serializes the World with bincode, prepends the header + metadata,
/// and writes everything to the file at `path`.
pub fn save(
    path: impl AsRef<Path>,
    world: &World,
    metadata: &VornMetadata,
) -> Result<(), FormatError> {
    let meta_bytes = bincode::serialize(metadata)?;
    let payload = bincode::serialize(world)?;

    let header = VornHeader::new(meta_bytes.len() as u32);
    let header_bytes = header.encode();

    let mut file = std::fs::File::create(path.as_ref())?;
    file.write_all(&header_bytes)?;
    file.write_all(&meta_bytes)?;
    file.write_all(&payload)?;
    file.flush()?;
    Ok(())
}

/// Saves a `World` inferring metadata from the World itself.
pub fn save_world(path: impl AsRef<Path>, world: &World) -> Result<(), FormatError> {
    let metadata = VornMetadata::new(
        world.settings.map_name.clone(),
        world.header.seed.clone(),
        world.header.date.clone(),
        env!("CARGO_PKG_VERSION").to_string(),
        Some(world.header.version.clone()),
    );
    save(path, world, &metadata)
}
