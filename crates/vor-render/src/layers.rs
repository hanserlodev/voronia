/// Bits de capas activables/desactivables.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerFlags {
    pub heightmap: bool,
    pub biomes: bool,
    pub rivers: bool,
    pub borders_state: bool,
    pub borders_province: bool,
    pub borders_culture: bool,
    pub burgs: bool,
    pub labels: bool,
}

impl Default for LayerFlags {
    fn default() -> Self {
        Self {
            heightmap: true,
            biomes: false,
            rivers: true,
            borders_state: true,
            borders_province: false,
            borders_culture: false,
            burgs: true,
            labels: true,
        }
    }
}

impl LayerFlags {
    /// Índices de capas adicionales registradas en el Renderer.
    /// Layer 0 es siempre heightmap. Los demás índices dependen del orden
    /// en que se registraron en `add_layer_mesh`.
    pub const LAYER_BIOMES: usize = 1;
    pub const LAYER_RIVERS: usize = 2;
    pub const LAYER_BORDER_STATE: usize = 3;
    pub const LAYER_BORDER_PROVINCE: usize = 4;
    pub const LAYER_BORDER_CULTURE: usize = 5;
    pub const LAYER_BURGS: usize = 6;
    pub const NUM_LAYERS: usize = 7;

    /// Retorna qué capas están activas como índices a dibujar.
    pub fn active_indices(&self) -> Vec<usize> {
        let mut indices = Vec::with_capacity(Self::NUM_LAYERS);
        if self.heightmap {
            indices.push(0);
        }
        if self.biomes {
            indices.push(Self::LAYER_BIOMES);
        }
        if self.rivers {
            indices.push(Self::LAYER_RIVERS);
        }
        if self.borders_state {
            indices.push(Self::LAYER_BORDER_STATE);
        }
        if self.borders_province {
            indices.push(Self::LAYER_BORDER_PROVINCE);
        }
        if self.borders_culture {
            indices.push(Self::LAYER_BORDER_CULTURE);
        }
        if self.burgs {
            indices.push(Self::LAYER_BURGS);
        }
        // labels se dibujan en egui, no acá
        indices
    }
}
