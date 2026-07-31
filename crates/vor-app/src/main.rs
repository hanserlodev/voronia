//! `vor-app` -- Voronia desktop application: window + wgpu + egui + viewer.
//!
//! Most of the logic lives in `App` (lib.rs). The binary initializes tracing,
//! parses a minimal CLI and starts it.

fn main() -> anyhow::Result<()> {
    vor_app::run_cli()
}
