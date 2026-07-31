<!-- Muchas gracias por contribuir a Voronia. Completá este template para acelerar la revisión. -->

## Resumen

¿Qué hace este PR? (una o dos frases)

Relacionado con: #issue (si aplica)

## Cambios

- [ ] `vor-core` (World Data Model)
- [ ] `vor-import` (parser .map/JSON)
- [ ] `vor-format` (serialización .vorn)
- [ ] `vor-sim` (motor de simulación)
- [ ] `vor-render` (pipeline wgpu)
- [ ] `vor-edit` (comandos + undo/redo)
- [ ] `vor-app` / `vor-cli`
- [ ] Docs (`docs/`, `SECURITY.md`, `.github/`)

## Verificación

<!-- Marcar lo que aplica. TODO debe estar verde antes del merge. -->

- [ ] `cargo test --workspace` verde
- [ ] `cargo clippy --workspace` sin warnings nuevos
- [ ] `cargo fmt --check` limpio
- [ ] Tests con **semilla fija** si toqué código generativo (misma seed = mismo output byte-idéntico)
- [ ] No agregué `println!` (usar `tracing`)
- [ ] `vor-render` no escribe al World Data Model (solo lee)
- [ ] No hay secretos ni claves en el diff
- [ ] Actualicé `docs/` y `.opencode/skills/voronia-dev/references/status.md` si aplica

## Screenshots / evidencia

<!-- Para cambios visuales de render, incluir screenshot del antes/después o comparación con Azgaar. -->

## Notas para el revisor

<!-- Decisiones de arquitectura, divergencias con Azgaar, pasos de prueba manuales. -->
