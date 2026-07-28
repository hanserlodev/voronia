use vor_render::layers::LayerFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
    Physical,
    HeightmapOnly,
    Biomes,
    Political,
    Cultural,
    Religions,
    Provinces,
    PlacesOfInterest,
    Goods,
    TradeAnimation,
    Military,
    Emblems,
    PureLandmass,
}

impl Preset {
    const ALL: [Preset; 13] = [
        Preset::Physical,
        Preset::HeightmapOnly,
        Preset::Biomes,
        Preset::Political,
        Preset::Cultural,
        Preset::Religions,
        Preset::Provinces,
        Preset::PlacesOfInterest,
        Preset::Goods,
        Preset::TradeAnimation,
        Preset::Military,
        Preset::Emblems,
        Preset::PureLandmass,
    ];

    fn label(&self) -> &'static str {
        match self {
            Preset::Physical => "Physical map",
            Preset::HeightmapOnly => "Heightmap",
            Preset::Biomes => "Biomes map",
            Preset::Political => "Political map",
            Preset::Cultural => "Cultural map",
            Preset::Religions => "Religions map",
            Preset::Provinces => "Provinces map",
            Preset::PlacesOfInterest => "Places of interest",
            Preset::Goods => "Goods map",
            Preset::TradeAnimation => "Trade animation",
            Preset::Military => "Military map",
            Preset::Emblems => "Emblems",
            Preset::PureLandmass => "Pure landmass",
        }
    }

    fn apply(&self, f: &mut LayerFlags) {
        *f = LayerFlags::default();
        match self {
            Preset::Physical => {
                f.heightmap = true;
                f.texture = false;
                f.relief = true;
                f.rivers = true;
                f.lakes = true;
                f.temperature = true;
                f.precipitation = true;
                f.ice = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::HeightmapOnly => {
                f.heightmap = true;
                f.relief = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::Biomes => {
                f.biomes = true;
                f.rivers = true;
                f.borders_state = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::Political => {
                f.states = true;
                f.provinces = true;
                f.burgs = true;
                f.borders_state = true;
                f.borders_province = true;
                f.rivers = true;
                f.population = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::Cultural => {
                f.cultures = true;
                f.states = true;
                f.religions = true;
                f.burgs = true;
                f.borders_state = true;
                f.borders_culture = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::Religions => {
                f.religions = true;
                f.states = true;
                f.borders_state = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::Provinces => {
                f.provinces = true;
                f.states = true;
                f.burgs = true;
                f.borders_state = true;
                f.borders_province = true;
                f.rivers = true;
                f.population = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::PlacesOfInterest => {
                f.burgs = true;
                f.markers = true;
                f.routes = true;
                f.goods = true;
                f.trade = true;
                f.states = true;
                f.borders_state = true;
                f.labels = true;
            }
            Preset::Goods => {
                f.goods = true;
                f.trade = true;
                f.routes = true;
                f.markers = true;
                f.wind_rose = true;
                f.scale_bar = true;
                f.labels = true;
            }
            Preset::TradeAnimation => {
                f.trade = true;
                f.routes = true;
                f.burgs = true;
                f.markers = true;
                f.grid = true;
            }
            Preset::Military => {
                f.states = true;
                f.burgs = true;
                f.rulers = true;
                f.icons = true;
                f.emblems = true;
                f.borders_state = true;
                f.labels = true;
            }
            Preset::Emblems => {
                f.emblems = true;
                f.states = true;
                f.burgs = true;
                f.labels = true;
            }
            Preset::PureLandmass => {
                f.texture = true;
                f.heightmap = true;
                f.relief = true;
                f.cells = true;
                f.grid = true;
                f.contours = true;
                f.coordinates = true;
            }
        }
    }
}

fn detect_preset(f: &LayerFlags) -> Option<Preset> {
    for p in &Preset::ALL {
        let mut candidate = LayerFlags::default();
        p.apply(&mut candidate);
        if fields_match(f, &candidate) {
            return Some(*p);
        }
    }
    None
}

fn fields_match(a: &LayerFlags, b: &LayerFlags) -> bool {
    a.texture == b.texture
        && a.heightmap == b.heightmap
        && a.relief == b.relief
        && a.cells == b.cells
        && a.grid == b.grid
        && a.contours == b.contours
        && a.coordinates == b.coordinates
        && a.lakes == b.lakes
        && a.rivers == b.rivers
        && a.temperature == b.temperature
        && a.precipitation == b.precipitation
        && a.ice == b.ice
        && a.biomes == b.biomes
        && a.goods == b.goods
        && a.routes == b.routes
        && a.states == b.states
        && a.provinces == b.provinces
        && a.zones == b.zones
        && a.cultures == b.cultures
        && a.religions == b.religions
        && a.population == b.population
        && a.burgs == b.burgs
        && a.markets == b.markets
        && a.trade == b.trade
        && a.borders_state == b.borders_state
        && a.borders_province == b.borders_province
        && a.borders_culture == b.borders_culture
        && a.markers == b.markers
        && a.icons == b.icons
        && a.emblems == b.emblems
        && a.rulers == b.rulers
        && a.labels == b.labels
        && a.wind_rose == b.wind_rose
        && a.scale_bar == b.scale_bar
        && a.vignette == b.vignette
}

pub fn show(ui: &mut egui::Ui, layer_flags: &mut LayerFlags) {
    ui.heading("Layers");

    // Preset selector
    let current_preset = detect_preset(layer_flags);
    let preset_label = current_preset.map(|p| p.label()).unwrap_or("Custom");
    let mut sel = current_preset;
    egui::ComboBox::from_id_salt("layer-preset")
        .selected_text(preset_label)
        .show_ui(ui, |ui| {
            for p in &Preset::ALL {
                let is_selected = current_preset == Some(*p);
                if ui.selectable_label(is_selected, p.label()).clicked() {
                    sel = Some(*p);
                }
            }
            let is_custom = current_preset.is_none();
            if ui.selectable_label(is_custom, "Custom").clicked() {
                sel = None;
            }
        });
    if let Some(p) = sel {
        if sel != current_preset {
            p.apply(layer_flags);
        }
    }

    ui.separator();

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id("layers-landmass"),
        true,
    )
    .show_header(ui, |ui| {
        ui.strong("Landmass");
    })
    .body(|ui| {
        checkbox(ui, layer_flags, "texture", |f| &mut f.texture);
        checkbox(ui, layer_flags, "heightmap", |f| &mut f.heightmap);
        checkbox(ui, layer_flags, "relief", |f| &mut f.relief);
        checkbox(ui, layer_flags, "cells", |f| &mut f.cells);
        checkbox(ui, layer_flags, "grid", |f| &mut f.grid);
        checkbox(ui, layer_flags, "contours", |f| &mut f.contours);
        checkbox(ui, layer_flags, "coordinates", |f| &mut f.coordinates);
    });

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id("layers-water-climate"),
        true,
    )
    .show_header(ui, |ui| {
        ui.strong("Water & Climate");
    })
    .body(|ui| {
        checkbox(ui, layer_flags, "lakes", |f| &mut f.lakes);
        checkbox(ui, layer_flags, "rivers", |f| &mut f.rivers);
        checkbox(ui, layer_flags, "temperature", |f| &mut f.temperature);
        checkbox(ui, layer_flags, "precipitation", |f| &mut f.precipitation);
        checkbox(ui, layer_flags, "ice", |f| &mut f.ice);
    });

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id("layers-biosphere"),
        true,
    )
    .show_header(ui, |ui| {
        ui.strong("Biosphere");
    })
    .body(|ui| {
        checkbox(ui, layer_flags, "biomes", |f| &mut f.biomes);
        checkbox(ui, layer_flags, "goods", |f| &mut f.goods);
        checkbox(ui, layer_flags, "routes", |f| &mut f.routes);
    });

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id("layers-human-geo"),
        true,
    )
    .show_header(ui, |ui| {
        ui.strong("Human Geography");
    })
    .body(|ui| {
        checkbox(ui, layer_flags, "states", |f| &mut f.states);
        checkbox(ui, layer_flags, "provinces", |f| &mut f.provinces);
        checkbox(ui, layer_flags, "zones", |f| &mut f.zones);
        checkbox(ui, layer_flags, "cultures", |f| &mut f.cultures);
        checkbox(ui, layer_flags, "religions", |f| &mut f.religions);
        checkbox(ui, layer_flags, "population", |f| &mut f.population);
        checkbox(ui, layer_flags, "burgs", |f| &mut f.burgs);
        checkbox(ui, layer_flags, "markets", |f| &mut f.markets);
        checkbox(ui, layer_flags, "trade", |f| &mut f.trade);
    });

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id("layers-borders"),
        true,
    )
    .show_header(ui, |ui| {
        ui.strong("Borders");
    })
    .body(|ui| {
        checkbox(ui, layer_flags, "state borders", |f| &mut f.borders_state);
        checkbox(ui, layer_flags, "province borders", |f| {
            &mut f.borders_province
        });
        checkbox(ui, layer_flags, "culture borders", |f| {
            &mut f.borders_culture
        });
    });

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.make_persistent_id("layers-overlay"),
        true,
    )
    .show_header(ui, |ui| {
        ui.strong("Overlay");
    })
    .body(|ui| {
        checkbox(ui, layer_flags, "markers", |f| &mut f.markers);
        checkbox(ui, layer_flags, "icons", |f| &mut f.icons);
        checkbox(ui, layer_flags, "emblems", |f| &mut f.emblems);
        checkbox(ui, layer_flags, "rulers", |f| &mut f.rulers);
        checkbox(ui, layer_flags, "labels", |f| &mut f.labels);
        checkbox(ui, layer_flags, "wind rose", |f| &mut f.wind_rose);
        checkbox(ui, layer_flags, "scale bar", |f| &mut f.scale_bar);
        checkbox(ui, layer_flags, "vignette", |f| &mut f.vignette);
    });
}

fn checkbox(
    ui: &mut egui::Ui,
    flags: &mut LayerFlags,
    label: &str,
    accessor: fn(&mut LayerFlags) -> &mut bool,
) {
    ui.checkbox(accessor(flags), label);
}
