# Changelog

Todas las versiones notables de Voronia se documentan en este archivo. El formato sigue [Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el proyecto respeta [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

### Added

- Conversión de altura a unidades reales (`Settings::height_m`, port de `getHeight` de Azgaar) aplicada
  al tooltip de hover y al tab Info — las alturas de tierra usan `(h - 18)^exponent` y el mar sale
  negativo, igual que FMG. (Doc: `docs/analysis/height-units-and-cell-tooltip.md`).
- Tooltip de celda al pasar el cursor (estilo Azgaar, centrado abajo).

### Changed

- Heightmap renderizado como **bandas de isoline rellenas** (`build_heightmap_band_mesh`, `isoline.rs`)
  con rampa Spectral/"bright" (`heightmap.rs`), paridad con el facetado de Azgaar; el océano queda
  excluido (`height < 20`).
- La capa de textura (canvas/paper) se dibuja **al inicio de Pass 1** como fondo (REPLACE) en vez de
  post-filtro; el océano usa pipeline alpha-blend (`ocean_pipeline`) y baja `alpha 0.55` para que el
  papel se vea tras el mar. (Doc: `docs/analysis/texture-background-render-fix.md`).
- Logs de `TextSystem` (`text.rs`) rebajados de `info!` a `trace!` (ruido).

### Fixed

- El océano ya no tapa la textura de papel (mezcla alfa en `renderer.rs`).

## [0.2.0] - 2026-07-30

Primer lanzamiento comunitario.

### Added

- GitHub community health files: `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `SECURITY.md`, templates de issues (bug + feature request) y PR.
- Fixture `Sorvik-2026-07-24-23-39.map` (Azgaar 1.138.0) committeado en `crates/vor-import/tests/reference/` para que los tests de handshake corran en cualquier checkout, sin depender de rutas locales.
- Documentación reorganizada en carpetas temáticas: `docs/phases/`, `docs/analysis/`, `docs/layers/`, `docs/plans/`, `docs/ui/`.

### Changed

- Versión del workspace: `0.1.0` → `0.2.0` (los 8 crates comparten la versión vía `version.workspace = true`).
- Todo el código, comentarios, docs y strings en runtime traducidos al inglés (repo público).
- Rutas absolutas locales (`/home/<user>/...`) eliminadas de tests y benchmarks; ahora usan rutas relativas a `CARGO_MANIFEST_DIR`.
- Plan maestro renumerado (§22–§28) tras eliminar la sección de integración con Atenea; el roadmap queda en §22.
- Metadata de versión en tests de `.vorn` ahora usa `env!("CARGO_PKG_VERSION")` en lugar de valores hardcodeados.

### Removed

- Referencias al proyecto Atenea del plan maestro, README y código.
- Skill local de desarrollo (`.opencode/`) del control de versiones — es configuración privada del maintainer.
