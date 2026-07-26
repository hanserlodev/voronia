//! `vor-app` -- aplicacion de escritorio de Voronia: ventana + wgpu + egui + visor.
//!
//! La mayor parte de la logica vive en `App` (lib.rs). El binario initra tracing,
//! parsea CLI minimal y la arranca.

fn main() -> anyhow::Result<()> {
    vor_app::run_cli()
}
