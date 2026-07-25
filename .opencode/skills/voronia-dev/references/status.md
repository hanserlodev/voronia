# Voronia — estado actual

> Este archivo se actualiza en cada sesión de trabajo donde pase algo relevante (ver protocolo de mantenimiento en `SKILL.md`). Mantenelo corto — es para orientarse rápido al empezar una sesión, no para llevar el historial completo (eso vive en `git log` y en el plan maestro).

**Última actualización**: 24 julio 2026 (configuración de OpenCode + regla de límite de contexto 160K)

## Fase actual del roadmap

**Fase 0 — Investigación y sentado de bases**: no iniciada.

Nada de código escrito todavía. Existe el plan maestro completo (`voronia-plan-proyecto.md`, §1–§29) y esta skill. El repo de Voronia en sí (workspace de Cargo, GitHub) todavía no se creó.

## Próximos pasos concretos (de la Fase 0, plan maestro §23)

- [ ] Clonar el repo de Azgaar y revisar `src/` para identificar el PRNG exacto que usa.
- [ ] Identificar el algoritmo exacto de Delaunay/Voronoi y de repacking grid→pack en el código fuente real.
- [ ] Exportar un mapa real de Azgaar (JSON completo) y diseccionar su estructura exacta.
- [ ] Crear el repo `voronia` bajo `hanserlodev`, licencia MIT, README con créditos a Azgaar.
- [ ] Setup del Cargo workspace vacío (estructura en `references/architecture.md`).
- [ ] Colocar esta skill en `.opencode/skills/voronia-dev/` dentro del repo recién creado (ver nota abajo).

## Decisiones tomadas (fuera de las que ya están en el plan maestro)

- Nombre del proyecto: **Voronia** (decidido 24 jul 2026, tras descartar `Worldforge`/`Terraforge` por colisión real con proyectos existentes — detalle en plan maestro §1.3).
- **Configuración de OpenCode** (24 jul 2026): creado `opencode.json` en la raíz del repo con `compaction.auto: true`, `compaction.prune: true`, `compaction.reserved: 16000` tokens, e `instructions` con español por defecto + referencia a esta skill. La compactación automática queda habilitada — el modelo GLM-5.2 vía NVIDIA tiene ventana nominal ~1M pero límite práctico ≈170K, así que el umbral operativo es 160K (ver nueva sección en `SKILL.md`).
- **Regla crítica de checkpoint de contexto** (24 jul 2026): al acercarse a 160K tokens el agente debe detener generación, escribir `agent_state_checkpoint.md` en la raíz, avisar a Hans y disparar compactación. Detalle completo en `SKILL.md` (sección "Límite de contexto y checkpoint de sesión"). `agent_state_checkpoint.md` agregado a `.gitignore` como ruta temporal no-commiteable.
- **Protocolo de reanudación** (24 jul 2026): al decir "continuar con el trabajo que se dejó / seguir donde lo dejamos / continuar con la Fase X", el agente lee `references/status.md` + `agent_state_checkpoint.md` si existe, **reconstruye el `todowrite`** ítem por ítem (mismo texto, orden, estado y prioridad), verifica con `git status` los archivos pendientes de commitear, confirma con Hans el punto exacto en 1-2 líneas, y ejecuta el próximo paso sugerido. Tras reanudar, borra el `agent_state_checkpoint.md`. Detalle en `SKILL.md` (subsección "Protocolo de reanudación").

## Bloqueos / cosas pendientes de confirmar

- Verificación final de disponibilidad exacta de `voronia` en GitHub y crates.io (chequeo rápido, no bloqueante para arrancar pero hacerlo antes de publicar nada).
- Ver plan maestro §26 (decisiones pendientes) para el resto: `.gmap` vs `.mapg`, alcance real de soporte a `.map` legacy, prioridad de la Fase 8.
- **Inconsistencia de `.gitignore` vs. skill (PENDIENTE DE CONFIRMACIÓN DE HANS)**: `.gitignore` línea `.opencode/` ignora TODA la carpeta, incluida `.opencode/skills/voronia-dev/` (la skill). Pero el protocolo de mantenimiento de `SKILL.md` (punto 4) dice que la skill debe commitearse junto con el código. Hoy la skill **no está trackeada** por git (`git check-ignore` la descarta). El archivo `.gitignore` actual comenta "skills live elsewhere", que parece ser una asunción vieja previa a meter la skill dentro del repo. Opciones: (a) cambiar `.opencode/` por reglas más finas que ignoren solo `node_modules/`/caches, (b) forzar el trackeo con `git add -f .opencode/skills/`, o (c) mover la skill fuera del repo (rompe el protocolo). esto lo dejé sin tocar porque excede el pedido de ahora — decidir antes de commitear la skill por primera vez.

## Nota sobre esta skill

Esta skill se escribió *antes* de que existiera el repo de Voronia (se generó en una conversación de planificación). Cuando se cree el repo real, esta carpeta (`voronia-dev/`) va dentro de `.opencode/skills/` en la raíz — eso la pone bajo control de versión junto con el código, que es la forma en que se mantiene sincronizada con cada cambio del proyecto (ver protocolo de mantenimiento en `SKILL.md`).
