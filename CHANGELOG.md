# Changelog

Todas las versiones notables de Voronia se documentan en este archivo. El formato sigue [Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el proyecto respeta [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

## [0.3.0] - 2026-08-16

Paridad ampliada de Human Geography y primeras capas económicas tipadas.

### Added

- Conversión de altura a unidades reales (`Settings::height_m`, port de `getHeight` de Azgaar) aplicada
  al tooltip de hover y al tab Info — las alturas de tierra usan `(h - 18)^exponent` y el mar sale
  negativo, igual que FMG. (Doc: `docs/analysis/height-units-and-cell-tooltip.md`).
- Tooltip de celda al pasar el cursor (estilo Azgaar, centrado abajo).
- Motor común de isolines para states, provinces, cultures, religions y zones.
- Generación nativa determinista de states, provinces, cultures y religions en `vor-sim`.
- Modelos tipados para goods, markets y trade deals, con importación desde slots `[41]`, `[42]` y `[43]`.
- Render inicial de goods, markets y trade.
- Barras de población rural y urbana, iconos de burgs y rutas suavizadas con Catmull-Rom.
- Diagnósticos de cobertura de landmask y paridad de Human Geography.

### Changed

- Heightmap renderizado como **bandas de isoline rellenas** (`build_heightmap_band_mesh`, `isoline.rs`)
  con rampa Spectral/"bright" (`heightmap.rs`), paridad con el facetado de Azgaar; el océano queda
  excluido (`height < 20`).
- La capa de climatología **Temperature** portada de Azgaar (`draw-temperature.ts`): bandas de isolines
  rellenas sobre la grilla (`temperature.rs`), `step = max(round(|min-max|/5), 1)`, cadena con
  `connect_vertices` (paridad exacta), relax 1-de-cada-4 + vértices de borde, y color
  `scheme(1 - (t-tMin)/delta)` con la rampa espectral (tMin=-50, delta=100); rectángulo base del mapa
  con el color de `minTemp`.
- La capa de precipitación portada de Azgaar (`drawPrecipitation` en `layers.js`): círculos azules
  `#003dff` centrados en cada celda de tierra (height ≥ 20) con precip > 0, radio
  `rn(sqrt(prec/4)/(cells/10000)⁰·²⁵, 2)` (modulador por densidad de celdas), en `precipitation.rs`.
- La capa de textura (canvas/paper) se dibuja **al inicio de Pass 1** como fondo (REPLACE) en vez de
  post-filtro; el océano usa pipeline alpha-blend (`ocean_pipeline`) y baja `alpha 0.55` para que el
  papel se vea tras el mar. (Doc: `docs/analysis/texture-background-render-fix.md`).
- Logs de `TextSystem` (`text.rs`) rebajados de `info!` a `trace!` (ruido).

### Fixed

- El océano ya no tapa la textura de papel (mezcla alfa en `renderer.rs`).
- Las capas semitransparentes usan composición alfa real.
- Los water gaps regionales usan la misma geometría raw que sus fills.
- Los borders usan aristas Voronoi compartidas sin duplicar segmentos.
- El exportador PNG incluye el stencil de landmask.
- Los índices de layers ya no desplazan los toggles al registrar capas económicas.

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
