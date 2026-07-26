//! `vor-cli` -- entry binario de Voronia.
//!
//! Delegar a `vor_app::run_cli` para el subcomando `viewer` (Fase 2). Mas
//! subcomandos headless en Fase 8 (export, batch, ...).

fn main() -> anyhow::Result<()> {
    // Solo subcomando `viewer` por ahora; pasarlo por LIB - simple argv.
    vor_app::run_cli()
}
