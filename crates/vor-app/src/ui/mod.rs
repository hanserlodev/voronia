mod about;
mod footer;
mod info;
mod layers;
mod modals;
mod options;
mod style;
mod tools;

pub use about::show as about_tab;
pub use footer::show as footer_bar;
pub use info::show as info_tab;
pub use layers::show as layers_tab;
pub use modals::{export_modal, load_modal, new_map_modal, save_modal};
pub use options::show as options_tab;
pub use style::show as style_tab;
pub use tools::show as tools_tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Layers,
    Info,
    Tools,
    Options,
    Style,
    About,
}

impl TabId {
    pub const ALL: [TabId; 6] = [
        TabId::Layers,
        TabId::Info,
        TabId::Tools,
        TabId::Options,
        TabId::Style,
        TabId::About,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            TabId::Layers => "Layers",
            TabId::Info => "Info",
            TabId::Tools => "Tools",
            TabId::Options => "Options",
            TabId::Style => "Style",
            TabId::About => "About",
        }
    }
}
