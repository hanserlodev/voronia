use vor_core::pack::Pack;

use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

/// Construye la malla de biomas: cada celda del pack se colorea según su bioma.
pub fn build_biome_mesh(pack: &Pack, biome_colors: &[[f32; 4]]) -> HeightmapMesh {
    let n_pack = pack.points_n();
    build_pack_mesh(&pack.vertices, n_pack, |p| {
        let bi = pack.cells.biome.get(p).copied().unwrap_or(0) as usize;
        biome_colors
            .get(bi)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0])
    })
}

/// Extrae colores de la lista de biomas (string hex `#rrggbb` → `[f32;4]` lineal).
pub fn biome_colors_from_catalog(biomes: &[vor_core::entities::biome::Biome]) -> Vec<[f32; 4]> {
    biomes
        .iter()
        .map(|b| hex_color_to_linear(&b.color))
        .collect()
}

pub fn hex_color_to_linear(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(hex.get(0..2).unwrap_or("00"), 16).unwrap_or(0);
    let g = u8::from_str_radix(hex.get(2..4).unwrap_or("00"), 16).unwrap_or(0);
    let b = u8::from_str_radix(hex.get(4..6).unwrap_or("00"), 16).unwrap_or(0);
    // sRGB → lineal aproximado (gamma 2.2)
    fn srgb_to_linear(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}
