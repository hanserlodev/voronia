---
name: "🐛 Bug report"
about: "Reportar un bug del motor Voronia"
title: "bug: [crate o módulo] descripción breve"
labels: ["bug"]
assignees: []
---

<!-- Antes de abrir: buscá si ya existe un issue similar y leé el plan maestro en docs/plan.md (§23) y docs/fase-*.md -->

## Descripción

¿Qué hace el bug? (una o dos frases)

## Pasos para reproducir

1. Abrir/importar: `...` (¿qué mapa? si es un `.map` de Azgaar, adjuntar o indicar nombre + seed)
2. Click en / ejecutar: `...`
3. Observar: `...`

## Comportamiento esperado

¿Qué debería pasar?

## Comportamiento actual

¿Qué pasa en realidad? Adjuntar screenshot si es posible (especialmente para bugs de render).

## Contexto

- **Crate / módulo**: p.ej. `vor-import`, `vor-render/src/coastline.rs`
- **Sistema**: Linux/macOS/Windows
- **GPU** (si es bug de render): p.ej. Intel UHD / NVIDIA 3050
- **Commit**: `git log -1` o versión
- **Comando usado**: p.ej. `cargo run -p vor-app -- path/al/mapa.map`

## Información adicional

<!-- Logs, stack traces, lo que ayude. -->

## Checklist

- [ ] Pude reproducirlo siempre / a veces (explicar)
- [ ] El mismo mapa y seed en Azgaar no muestra el problema (si aplica)
