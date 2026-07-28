#[derive(Debug, Clone)]
pub struct LayerFlags {
    // Landmass
    pub texture: bool,
    pub heightmap: bool,
    pub relief: bool,
    pub cells: bool,
    pub grid: bool,
    pub contours: bool,
    pub coordinates: bool,
    // Water & climate
    pub lakes: bool,
    pub rivers: bool,
    pub temperature: bool,
    pub precipitation: bool,
    pub ice: bool,
    // Biosphere
    pub biomes: bool,
    pub goods: bool,
    pub routes: bool,
    // Human geography
    pub states: bool,
    pub provinces: bool,
    pub zones: bool,
    pub cultures: bool,
    pub religions: bool,
    pub population: bool,
    pub burgs: bool,
    pub markets: bool,
    pub trade: bool,
    // Overlay
    pub borders_state: bool,
    pub borders_province: bool,
    pub borders_culture: bool,
    pub markers: bool,
    pub icons: bool,
    pub emblems: bool,
    pub rulers: bool,
    pub labels: bool,
    pub wind_rose: bool,
    pub scale_bar: bool,
    pub vignette: bool,
}

impl Default for LayerFlags {
    fn default() -> Self {
        Self {
            texture: false,
            heightmap: false,
            relief: false,
            cells: false,
            grid: false,
            contours: false,
            coordinates: false,
            lakes: true,
            rivers: true,
            temperature: false,
            precipitation: false,
            ice: false,
            biomes: false,
            goods: false,
            routes: false,
            states: false,
            provinces: false,
            zones: false,
            cultures: false,
            religions: false,
            population: false,
            burgs: false,
            markets: false,
            trade: false,
            borders_state: false,
            borders_province: false,
            borders_culture: false,
            markers: false,
            icons: false,
            emblems: false,
            rulers: false,
            labels: true,
            wind_rose: true,
            scale_bar: true,
            vignette: false,
        }
    }
}

impl LayerFlags {
    // Draw order (bottom → top):
    //   0: landmass (white base, always drawn)
    //   1: heightmap color (colored elevation overlay)
    //   2: relief    (landmass shading)
    //   3: biomes    (landmass color)
    //   4: temperature, 5: precipitation, 6: ice (climate)
    //   7: lakes, 8: rivers (water)
    //   9: state_fill, 10: province_fill, 11: culture_fill, 12: religion_fill (human geo fills)
    //  13: population, 14: zones (human geo overlays)
    //  15: border_state, 16: border_province, 17: border_culture (borders on top)
    //  18: burgs (markers on top)
    pub const LAYER_HEIGHTMAP: usize = 1;
    pub const LAYER_RELIEF: usize = 2;
    pub const LAYER_BIOMES: usize = 3;
    pub const LAYER_TEMPERATURE: usize = 4;
    pub const LAYER_PRECIPITATION: usize = 5;
    pub const LAYER_ICE: usize = 6;
    pub const LAYER_LAKES: usize = 7;
    pub const LAYER_RIVERS: usize = 8;
    pub const LAYER_STATE_FILL: usize = 9;
    pub const LAYER_PROVINCE_FILL: usize = 10;
    pub const LAYER_CULTURE_FILL: usize = 11;
    pub const LAYER_RELIGION_FILL: usize = 12;
    pub const LAYER_POPULATION: usize = 13;
    pub const LAYER_ZONES: usize = 14;
    pub const LAYER_BORDER_STATE: usize = 15;
    pub const LAYER_BORDER_PROVINCE: usize = 16;
    pub const LAYER_BORDER_CULTURE: usize = 17;
    pub const LAYER_BURGS: usize = 18;
    pub const NUM_LAYERS: usize = 19;

    pub fn active_indices(&self) -> Vec<usize> {
        let mut indices = Vec::with_capacity(Self::NUM_LAYERS);
        // Landmass base (always drawn)
        indices.push(0);
        // Heightmap color overlay
        if self.heightmap {
            indices.push(Self::LAYER_HEIGHTMAP);
        }
        // Shading
        if self.relief {
            indices.push(Self::LAYER_RELIEF);
        }
        if self.biomes {
            indices.push(Self::LAYER_BIOMES);
        }
        // Climate
        if self.temperature {
            indices.push(Self::LAYER_TEMPERATURE);
        }
        if self.precipitation {
            indices.push(Self::LAYER_PRECIPITATION);
        }
        if self.ice {
            indices.push(Self::LAYER_ICE);
        }
        // Water
        if self.lakes {
            indices.push(Self::LAYER_LAKES);
        }
        if self.rivers {
            indices.push(Self::LAYER_RIVERS);
        }
        // Human geography fills
        if self.states {
            indices.push(Self::LAYER_STATE_FILL);
        }
        if self.provinces {
            indices.push(Self::LAYER_PROVINCE_FILL);
        }
        if self.cultures {
            indices.push(Self::LAYER_CULTURE_FILL);
        }
        if self.religions {
            indices.push(Self::LAYER_RELIGION_FILL);
        }
        if self.population {
            indices.push(Self::LAYER_POPULATION);
        }
        if self.zones {
            indices.push(Self::LAYER_ZONES);
        }
        // Borders & markers on top
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
        indices
    }
}
