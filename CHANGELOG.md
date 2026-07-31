# Changelog

Todas las versiones notables de Voronia se documentan en este archivo. El formato sigue [Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el proyecto respeta [Semantic Versioning](https://semver.org/lang/es/).

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
