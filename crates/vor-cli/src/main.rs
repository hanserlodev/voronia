//! `vor-cli` -- Voronia binary entry point.
//!
//! Delegates to `vor_app::run_cli` for the `viewer` subcommand (Phase 2). More
//! headless subcommands in Phase 8 (export, batch, ...).

fn main() -> anyhow::Result<()> {
    // Only the `viewer` subcommand for now; pass it through the LIB - simple argv.
    vor_app::run_cli()
}
