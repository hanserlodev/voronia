#[derive(Debug, Clone)]
pub struct LayerFlags {
    // Landmass
    pub texture: bool,
    pub heightmap: bool,
    pub relief: bool,
    pub cells: bool,
    pub grid: bool,
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
            labels: false,
            wind_rose: true,
            scale_bar: true,
            vignette: false,
        }
    }
}

/// A single entry of the per-frame draw sequence: either a registered mesh
/// layer (`Renderer::draw_layer`), a line layer (`Renderer::draw_line_layer`),
/// or the special `#texture` overlay (own pipeline on `State`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawItem {
    Mesh(usize),
    Line(usize),
    /// FMG `#texture`: paper texture masked to land, above `#landmass`.
    Texture,
    /// FMG `#terrain`: relief icons (atlas overlay on `State`).
    Relief,
    /// FMG `#goodsIcons`/`#goodsBurgs` symbol quads (atlas overlay).
    GoodsIcons,
}

/// Runtime-registered layer indices that are not part of the fixed
/// `LAYER_*` constant block (line layers + economy meshes). The app fills
/// this after registering everything with the renderer.
#[derive(Debug, Clone, Copy)]
pub struct DynamicLayerIds {
    pub cells_line: usize,
    pub grid_line: usize,
    pub coordinates_line: usize,
    pub goods_cells: usize,
    pub goods_icons: usize,
    pub market_fill: usize,
    pub market_border: usize,
    pub market_center: usize,
    pub trade: usize,
    /// FMG `#coastline`: offset drop-shadow approximation (sea features).
    pub coastline_shadow: usize,
    /// FMG `#coastline`: hybrid-path strokes (sea + lake islands).
    pub coastline_stroke: usize,
    /// FMG `#oceanLayers`: bathymetry contour rings.
    pub ocean_bathymetry: usize,
    /// FMG `#lakes`: shore strokes of the styled subgroups.
    pub lake_stroke: usize,
    /// FMG `#biomes`: coastal water-gap stroke (width 3, biome color).
    pub biome_gap: usize,
    /// FMG `#routes` subgroup meshes (tessellated strokes, exact dashes).
    pub routes_roads: usize,
    pub routes_trails: usize,
    pub routes_searoutes: usize,
    /// FMG `#ice`: dropShadow01 approximation (offset black copy).
    pub ice_shadow: usize,
    /// FMG `#ice`: shore strokes.
    pub ice_stroke: usize,
    /// FMG `#goodsBurgs`: production plates (rects + circles mesh).
    pub goods_burgs: usize,
}

/// Per-frame draw options that depend on view state rather than flags.
#[derive(Debug, Clone, Copy)]
pub struct DrawOptions {
    /// FMG auto-filter (`invokeActiveZooming`): the sea drop-shadow is only
    /// applied while the zoom scale is ≤ 1.5.
    pub coastline_shadow: bool,
}

impl LayerFlags {
    // Fixed mesh layer indices, numbered to match the FMG `#viewbox` child
    // order restricted to implemented fill/icon layers (public/main.js):
    //   #landmass < #terrs < #lakes < #biomes < (#cells) < (#gridOverlay)
    //   < (#coordinates) < #rivers < #terrain < #relig < #cults < #regions
    //   < #provs < #zones < #borders < (#routes) < #temperature < (#coastline)
    //   < #ice < #goods < #markets < #tradeAnimation < #prec < #population
    //   < (#emblems) < #icons
    //
    // Draw order (bottom → top), FMG v1.138 parity:
    //   0: landmass (fractal landmass ∪ land cells, always drawn, stamps mask)
    //   1: heightmap (#terrs)
    //   2: lakes, 3: biomes
    //   [cells/grid/coordinates lines]
    //   4: rivers, 5: relief icons (#terrain)
    //   6: religion fill, 7: culture fill
    //   8: state fill, 9: province fill, 10: zones
    //  11: border_state, 12: border_province, 13: border_culture
    //   [routes line]
    //  14: temperature
    //   [coastline strokes — slot reserved between temperature and ice]
    //  15: ice
    //   [goods / markets / trade economy meshes]
    //  16: precipitation, 17: population
    //  18: burgs (#icons)
    pub const LAYER_HEIGHTMAP: usize = 1;
    pub const LAYER_LAKES: usize = 2;
    pub const LAYER_BIOMES: usize = 3;
    pub const LAYER_RIVERS: usize = 4;
    pub const LAYER_RELIEF: usize = 5;
    pub const LAYER_RELIGION_FILL: usize = 6;
    pub const LAYER_CULTURE_FILL: usize = 7;
    pub const LAYER_STATE_FILL: usize = 8;
    pub const LAYER_PROVINCE_FILL: usize = 9;
    pub const LAYER_ZONES: usize = 10;
    pub const LAYER_BORDER_STATE: usize = 11;
    pub const LAYER_BORDER_PROVINCE: usize = 12;
    pub const LAYER_BORDER_CULTURE: usize = 13;
    pub const LAYER_TEMPERATURE: usize = 14;
    pub const LAYER_ICE: usize = 15;
    pub const LAYER_PRECIPITATION: usize = 16;
    pub const LAYER_POPULATION: usize = 17;
    pub const LAYER_BURGS: usize = 18;
    pub const NUM_LAYERS: usize = 19;

    /// Full ordered draw sequence (bottom → top) matching the FMG `#viewbox`
    /// z-order for every layer currently implemented. Both the interactive
    /// frame and the PNG export must iterate exactly this list.
    pub fn draw_sequence(&self, dyn_ids: &DynamicLayerIds, opts: &DrawOptions) -> Vec<DrawItem> {
        use DrawItem::{Line, Mesh};
        let mut seq = Vec::with_capacity(Self::NUM_LAYERS);
        // #ocean group: bathymetry rings (the base color and the tile pattern
        // are drawn as fixed quads right before this sequence).
        seq.push(Mesh(dyn_ids.ocean_bathymetry));
        // #landmass (always drawn, stamps the stencil mask)
        seq.push(Mesh(0));
        // #texture: FMG draws it above the landmass fill, masked to land
        if self.texture {
            seq.push(DrawItem::Texture);
        }
        // #terrs
        if self.heightmap {
            seq.push(Mesh(Self::LAYER_HEIGHTMAP));
        }
        // #lakes: subgroup fills, then their shore strokes on top
        if self.lakes {
            seq.push(Mesh(Self::LAYER_LAKES));
            seq.push(Mesh(dyn_ids.lake_stroke));
        }
        // #biomes: isoline fills + the coastal gap stroke on top
        if self.biomes {
            seq.push(Mesh(Self::LAYER_BIOMES));
            seq.push(Mesh(dyn_ids.biome_gap));
        }
        // #cells, #gridOverlay, #coordinates
        if self.cells {
            seq.push(Line(dyn_ids.cells_line));
        }
        if self.grid {
            seq.push(Line(dyn_ids.grid_line));
        }
        if self.coordinates {
            seq.push(Line(dyn_ids.coordinates_line));
        }
        // #rivers, #terrain (relief icons render as a special atlas overlay)
        if self.rivers {
            seq.push(Mesh(Self::LAYER_RIVERS));
        }
        if self.relief {
            seq.push(DrawItem::Relief);
        }
        // #relig, #cults
        if self.religions {
            seq.push(Mesh(Self::LAYER_RELIGION_FILL));
        }
        if self.cultures {
            seq.push(Mesh(Self::LAYER_CULTURE_FILL));
        }
        // #regions, #provs, #zones
        if self.states {
            seq.push(Mesh(Self::LAYER_STATE_FILL));
        }
        if self.provinces {
            seq.push(Mesh(Self::LAYER_PROVINCE_FILL));
        }
        if self.zones {
            seq.push(Mesh(Self::LAYER_ZONES));
        }
        // #borders
        if self.borders_state {
            seq.push(Mesh(Self::LAYER_BORDER_STATE));
        }
        if self.borders_province {
            seq.push(Mesh(Self::LAYER_BORDER_PROVINCE));
        }
        if self.borders_culture {
            seq.push(Mesh(Self::LAYER_BORDER_CULTURE));
        }
        // #routes: subgroup meshes in FMG creation order
        if self.routes {
            seq.push(Mesh(dyn_ids.routes_roads));
            seq.push(Mesh(dyn_ids.routes_trails));
            seq.push(Mesh(dyn_ids.routes_searoutes));
        }
        // #temperature
        if self.temperature {
            seq.push(Mesh(Self::LAYER_TEMPERATURE));
        }
        // #coastline (always visible in FMG — there is no Layers toggle for
        // it): shadow first, then the strokes paint over it.
        if opts.coastline_shadow {
            seq.push(Mesh(dyn_ids.coastline_shadow));
        }
        seq.push(Mesh(dyn_ids.coastline_stroke));
        // #ice: shadow (filter dropShadow01) → fill → stroke
        if self.ice {
            seq.push(Mesh(dyn_ids.ice_shadow));
            seq.push(Mesh(Self::LAYER_ICE));
            seq.push(Mesh(dyn_ids.ice_stroke));
        }
        // #goods: production cells → symbol quads overlay → burg plates
        if self.goods {
            seq.push(Mesh(dyn_ids.goods_cells));
            seq.push(DrawItem::GoodsIcons);
            seq.push(Mesh(dyn_ids.goods_burgs));
        }
        // #markets
        if self.markets {
            seq.push(Mesh(dyn_ids.market_fill));
            seq.push(Mesh(dyn_ids.market_border));
            seq.push(Mesh(dyn_ids.market_center));
        }
        // #tradeAnimation
        if self.trade {
            seq.push(Mesh(dyn_ids.trade));
        }
        // #prec, #population
        if self.precipitation {
            seq.push(Mesh(Self::LAYER_PRECIPITATION));
        }
        if self.population {
            seq.push(Mesh(Self::LAYER_POPULATION));
        }
        // #emblems has no mesh yet; #icons (burgs) is the top implemented layer.
        if self.burgs {
            seq.push(Mesh(Self::LAYER_BURGS));
        }
        seq
    }

    /// Deprecated mesh-only ordering kept for callers that cannot draw line
    /// layers. Prefer [`Self::draw_sequence`].
    pub fn active_indices(&self) -> Vec<usize> {
        let mut indices = Vec::with_capacity(Self::NUM_LAYERS);
        // Landmass base (always drawn)
        indices.push(0);
        if self.heightmap {
            indices.push(Self::LAYER_HEIGHTMAP);
        }
        if self.lakes {
            indices.push(Self::LAYER_LAKES);
        }
        if self.biomes {
            indices.push(Self::LAYER_BIOMES);
        }
        if self.rivers {
            indices.push(Self::LAYER_RIVERS);
        }
        if self.relief {
            indices.push(Self::LAYER_RELIEF);
        }
        if self.religions {
            indices.push(Self::LAYER_RELIGION_FILL);
        }
        if self.cultures {
            indices.push(Self::LAYER_CULTURE_FILL);
        }
        if self.states {
            indices.push(Self::LAYER_STATE_FILL);
        }
        if self.provinces {
            indices.push(Self::LAYER_PROVINCE_FILL);
        }
        if self.zones {
            indices.push(Self::LAYER_ZONES);
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
        if self.temperature {
            indices.push(Self::LAYER_TEMPERATURE);
        }
        if self.ice {
            indices.push(Self::LAYER_ICE);
        }
        if self.precipitation {
            indices.push(Self::LAYER_PRECIPITATION);
        }
        if self.population {
            indices.push(Self::LAYER_POPULATION);
        }
        if self.burgs {
            indices.push(Self::LAYER_BURGS);
        }
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixed layer constants must be exactly 1..=NUM_LAYERS-1 in order:
    /// `draw_layer(layer_index)` looks up `self.layers[layer_index - 1]`, so a
    /// gap or a wrong constant silently draws the WRONG mesh (the toggles would
    /// activate the wrong layer). This test guards that mapping.
    #[test]
    fn layer_constants_are_contiguous_and_match_num_layers() {
        let expected = [
            LayerFlags::LAYER_HEIGHTMAP,
            LayerFlags::LAYER_LAKES,
            LayerFlags::LAYER_BIOMES,
            LayerFlags::LAYER_RIVERS,
            LayerFlags::LAYER_RELIEF,
            LayerFlags::LAYER_RELIGION_FILL,
            LayerFlags::LAYER_CULTURE_FILL,
            LayerFlags::LAYER_STATE_FILL,
            LayerFlags::LAYER_PROVINCE_FILL,
            LayerFlags::LAYER_ZONES,
            LayerFlags::LAYER_BORDER_STATE,
            LayerFlags::LAYER_BORDER_PROVINCE,
            LayerFlags::LAYER_BORDER_CULTURE,
            LayerFlags::LAYER_TEMPERATURE,
            LayerFlags::LAYER_ICE,
            LayerFlags::LAYER_PRECIPITATION,
            LayerFlags::LAYER_POPULATION,
            LayerFlags::LAYER_BURGS,
        ];
        for (i, &constant) in expected.iter().enumerate() {
            assert_eq!(
                constant,
                i + 1,
                "LAYER_* constants must be contiguous starting at 1 (got {} at position {})",
                constant,
                i
            );
        }
        assert_eq!(expected.len(), LayerFlags::NUM_LAYERS - 1);
    }

    fn dyn_ids() -> DynamicLayerIds {
        DynamicLayerIds {
            cells_line: 100,
            grid_line: 101,
            coordinates_line: 102,
            goods_cells: 104,
            goods_icons: 105,
            market_fill: 106,
            market_border: 107,
            market_center: 108,
            trade: 109,
            coastline_shadow: 110,
            coastline_stroke: 111,
            ocean_bathymetry: 112,
            lake_stroke: 113,
            biome_gap: 116,
            routes_roads: 117,
            routes_trails: 118,
            routes_searoutes: 119,
            goods_burgs: 120,
            ice_shadow: 114,
            ice_stroke: 115,
        }
    }

    fn opts(shadow: bool) -> DrawOptions {
        DrawOptions {
            coastline_shadow: shadow,
        }
    }

    /// With every flag on, the draw sequence must reproduce the FMG `#viewbox`
    /// child order restricted to implemented layers:
    /// landmass → terrs → lakes → biomes → cells → gridOverlay → coordinates
    /// → rivers → terrain → relig → cults → regions → provs → zones → borders
    /// → routes → temperature → ice → goods → markets → tradeAnimation → prec
    /// → population → icons(burgs).
    #[test]
    fn draw_sequence_matches_fmg_viewbox_order() {
        let mut flags = LayerFlags {
            texture: false,
            wind_rose: false,
            scale_bar: false,
            vignette: false,
            labels: false,
            markers: false,
            icons: false,
            emblems: false,
            rulers: false,
            ..LayerFlags::default()
        };
        flags.heightmap = true;
        flags.relief = true;
        flags.cells = true;
        flags.grid = true;
        flags.coordinates = true;
        flags.biomes = true;
        flags.temperature = true;
        flags.precipitation = true;
        flags.ice = true;
        flags.goods = true;
        flags.routes = true;
        flags.states = true;
        flags.provinces = true;
        flags.zones = true;
        flags.cultures = true;
        flags.religions = true;
        flags.population = true;
        flags.burgs = true;
        flags.markets = true;
        flags.trade = true;
        flags.borders_state = true;
        flags.borders_province = true;
        flags.borders_culture = true;

        let ids = dyn_ids();
        let seq = flags.draw_sequence(&ids, &opts(true));
        let expected = vec![
            DrawItem::Mesh(ids.ocean_bathymetry),
            DrawItem::Mesh(0),
            DrawItem::Mesh(LayerFlags::LAYER_HEIGHTMAP),
            DrawItem::Mesh(LayerFlags::LAYER_LAKES),
            DrawItem::Mesh(ids.lake_stroke),
            DrawItem::Mesh(LayerFlags::LAYER_BIOMES),
            DrawItem::Mesh(ids.biome_gap),
            DrawItem::Line(ids.cells_line),
            DrawItem::Line(ids.grid_line),
            DrawItem::Line(ids.coordinates_line),
            DrawItem::Mesh(LayerFlags::LAYER_RIVERS),
            DrawItem::Relief,
            DrawItem::Mesh(LayerFlags::LAYER_RELIGION_FILL),
            DrawItem::Mesh(LayerFlags::LAYER_CULTURE_FILL),
            DrawItem::Mesh(LayerFlags::LAYER_STATE_FILL),
            DrawItem::Mesh(LayerFlags::LAYER_PROVINCE_FILL),
            DrawItem::Mesh(LayerFlags::LAYER_ZONES),
            DrawItem::Mesh(LayerFlags::LAYER_BORDER_STATE),
            DrawItem::Mesh(LayerFlags::LAYER_BORDER_PROVINCE),
            DrawItem::Mesh(LayerFlags::LAYER_BORDER_CULTURE),
            DrawItem::Mesh(ids.routes_roads),
            DrawItem::Mesh(ids.routes_trails),
            DrawItem::Mesh(ids.routes_searoutes),
            DrawItem::Mesh(LayerFlags::LAYER_TEMPERATURE),
            // #coastline: shadow (auto-filter on at scale ≤ 1.5) + strokes.
            DrawItem::Mesh(ids.coastline_shadow),
            DrawItem::Mesh(ids.coastline_stroke),
            DrawItem::Mesh(ids.ice_shadow),
            DrawItem::Mesh(LayerFlags::LAYER_ICE),
            DrawItem::Mesh(ids.ice_stroke),
            DrawItem::Mesh(ids.goods_cells),
            DrawItem::GoodsIcons,
            DrawItem::Mesh(ids.goods_burgs),
            DrawItem::Mesh(ids.market_fill),
            DrawItem::Mesh(ids.market_border),
            DrawItem::Mesh(ids.market_center),
            DrawItem::Mesh(ids.trade),
            DrawItem::Mesh(LayerFlags::LAYER_PRECIPITATION),
            DrawItem::Mesh(LayerFlags::LAYER_POPULATION),
            DrawItem::Mesh(LayerFlags::LAYER_BURGS),
        ];
        assert_eq!(seq, expected);
    }

    /// Landmass and the coastline strokes are always drawn, even when every
    /// toggle is off (FMG has no Layers toggle for `#coastline`).
    #[test]
    fn draw_sequence_always_contains_landmass() {
        let flags = LayerFlags {
            lakes: false,
            rivers: false,
            wind_rose: false,
            scale_bar: false,
            ..LayerFlags::default()
        };
        let ids = dyn_ids();
        let seq = flags.draw_sequence(&ids, &opts(false));
        assert_eq!(
            seq,
            vec![
                DrawItem::Mesh(ids.ocean_bathymetry),
                DrawItem::Mesh(0),
                DrawItem::Mesh(ids.coastline_stroke)
            ]
        );
    }

    /// The coastline shadow is dropped above the FMG auto-filter threshold
    /// (scale > 1.5), but the strokes themselves are always drawn.
    #[test]
    fn draw_sequence_coastline_respects_shadow_option() {
        let flags = LayerFlags {
            lakes: false,
            rivers: false,
            wind_rose: false,
            scale_bar: false,
            ..LayerFlags::default()
        };
        let ids = dyn_ids();
        let seq = flags.draw_sequence(&ids, &opts(true));
        assert_eq!(
            seq,
            vec![
                DrawItem::Mesh(ids.ocean_bathymetry),
                DrawItem::Mesh(0),
                DrawItem::Mesh(ids.coastline_shadow),
                DrawItem::Mesh(ids.coastline_stroke)
            ]
        );
        let seq = flags.draw_sequence(&ids, &opts(false));
        assert_eq!(
            seq,
            vec![
                DrawItem::Mesh(ids.ocean_bathymetry),
                DrawItem::Mesh(0),
                DrawItem::Mesh(ids.coastline_stroke)
            ]
        );
    }
}
