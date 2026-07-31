pub mod error;
pub mod header;
pub mod load;
pub mod save;

pub use error::FormatError;
pub use header::{VornHeader, VornMetadata, VORN_FORMAT_VERSION, VORN_MAGIC};
pub use load::load;
pub use save::save;

#[cfg(test)]
mod tests {
    use vor_core::world::World;

    use crate::header::VornMetadata;
    use crate::VORN_FORMAT_VERSION;

    #[test]
    fn roundtrip_default_world() {
        let world = World::default();
        let meta = VornMetadata::new("test", "12345", "2026-7-27", env!("CARGO_PKG_VERSION"), None::<&str>);

        let bytes = {
            let meta_bytes = bincode::serialize(&meta).unwrap();
            let payload = bincode::serialize(&world).unwrap();

            let mut header = [0u8; 16];
            header[0..4].copy_from_slice(&crate::VORN_MAGIC);
            header[4..6].copy_from_slice(&VORN_FORMAT_VERSION.to_le_bytes());
            header[6..10].copy_from_slice(&(meta_bytes.len() as u32).to_le_bytes());
            header[10] = 0;

            let mut buf = Vec::new();
            buf.extend_from_slice(&header);
            buf.extend_from_slice(&meta_bytes);
            buf.extend_from_slice(&payload);
            buf
        };

        // Read header
        let mut header_buf = [0u8; 16];
        header_buf.copy_from_slice(&bytes[0..16]);
        let decoded = crate::header::VornHeader::decode(&header_buf).unwrap();
        assert_eq!(decoded.format_version, VORN_FORMAT_VERSION);
        assert_eq!(decoded.compression, 0);

        let meta_offset = 16;
        let meta_end = meta_offset + decoded.metadata_len as usize;
        let meta_bytes = &bytes[meta_offset..meta_end];
        let loaded_meta: VornMetadata = bincode::deserialize(meta_bytes).unwrap();
        assert_eq!(loaded_meta.map_name, "test");

        let payload = &bytes[meta_end..];
        let loaded_world: World = bincode::deserialize(payload).unwrap();
        assert_eq!(loaded_world, world);
    }

    #[test]
    fn roundtrip_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_roundtrip.vorn");

        let world = World::default();
        let meta = VornMetadata::new(
            "roundtrip_test",
            "99999",
            "2026-7-27",
            env!("CARGO_PKG_VERSION"),
            Some("1.138.0"),
        );

        crate::save::save(&path, &world, &meta).unwrap();
        assert!(path.exists());

        let (loaded_world, loaded_meta) = crate::load::load(&path).unwrap();
        assert_eq!(loaded_world, world);
        assert_eq!(loaded_meta.map_name, "roundtrip_test");
        assert_eq!(loaded_meta.azgaar_version, Some("1.138.0".to_string()));

        let loaded_meta2 = crate::load::load_metadata(&path).unwrap();
        assert_eq!(loaded_meta2.map_name, "roundtrip_test");
        assert_eq!(loaded_meta2.seed, "99999");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn invalid_magic() {
        let dir = std::env::temp_dir();
        let path = dir.join("bad_magic.vorn");

        let mut bad = vec![0u8; 32];
        bad[0..4].copy_from_slice(b"BAMN");
        std::fs::write(&path, &bad).unwrap();

        let err = crate::load::load(&path).unwrap_err();
        match err {
            crate::FormatError::InvalidMagic { found } => {
                assert_eq!(found, *b"BAMN");
            }
            other => panic!("expected InvalidMagic, got {other}"),
        }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unsupported_version() {
        let dir = std::env::temp_dir();
        let path = dir.join("bad_version.vorn");

        let mut bad = vec![0u8; 32];
        bad[0..4].copy_from_slice(b"VORN");
        bad[4..6].copy_from_slice(&99u16.to_le_bytes());
        std::fs::write(&path, &bad).unwrap();

        let err = crate::load::load(&path).unwrap_err();
        match err {
            crate::FormatError::UnsupportedVersion(v) => {
                assert_eq!(v, 99);
            }
            other => panic!("expected UnsupportedVersion, got {other}"),
        }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn save_world_convenience() {
        let dir = std::env::temp_dir();
        let path = dir.join("save_world.vorn");

        let mut world = World::default();
        world.header.seed = "convenience_seed".to_string();
        world.settings.map_name = "Convenience Test".to_string();

        crate::save::save_world(&path, &world).unwrap();

        let (loaded, meta) = crate::load::load(&path).unwrap();
        assert_eq!(loaded.header.seed, "convenience_seed");
        assert_eq!(meta.map_name, "Convenience Test");
        assert_eq!(meta.azgaar_version, Some("".to_string()));

        std::fs::remove_file(&path).ok();
    }
}
