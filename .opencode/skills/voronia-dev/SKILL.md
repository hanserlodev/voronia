---
name: voronia-dev
description: Contexto experto y siempre vigente del proyecto Voronia (motor nativo en Rust + wgpu para generación y edición de mundos de fantasía, inspirado en Azgaar's Fantasy Map Generator). Usar esta skill SIEMPRE que se trabaje dentro del repo de Voronia o se hable de él — escribir o revisar código Rust de cualquier crate del workspace, tocar el World Data Model, el importador de mapas de Azgaar (.map/JSON), el formato .gmap, el renderer wgpu, el motor de simulación procedural (heightmap, hidrología, culturas, estados, religiones, rutas), la UI egui, o cualquier decisión de arquitectura. También cuando se mencione "Voronia", se pida seguir con "el motor de mapas" o "el world engine", o continuar el roadmap del proyecto, aunque no se use el nombre exacto. Incluye una regla crítica de gestión de contexto: umbral operativo 160K tokens (límite NVIDIA GLM-5.2 ≈170K) con checkpoint obligatorio en agent_state_checkpoint.md y compactación automática. Esta skill es un documento vivo — léela Y actualízala en cada sesión de trabajo relevante (protocolo al final del archivo).
---

# Voronia — skill de desarrollo

## Qué es Voronia

Motor nativo (Rust + wgpu), acelerado por GPU, para generar y editar mundos de fantasía. Reescrito desde cero, no un port — pero basado en la lógica y el modelo de datos de **Azgaar's Fantasy Map Generator** (MIT), con atribución explícita. Puede importar mapas generados en Azgaar (`.map`/JSON), los convierte a un formato binario propio (`.gmap`), y va camino a reimplementar y expandir toda la generación procedural de forma nativa. Es de Hans, es de código abierto, y eventualmente se integra con Atenea (su asistente IA local).

## Antes de tocar nada: dónde está la verdad

Este repo tiene tres capas de documentación, cada una con un propósito distinto — no las confundas ni dupliques contenido entre ellas:

1. **El plan maestro del proyecto** (probablemente `docs/plan.md`, `PLAN.md` o `voronia-plan-proyecto.md` en la raíz — si no lo encontrás en una ruta obvia, buscalo antes de asumir que no existe). Es el documento de referencia completo: visión, arquitectura, modelo de datos detallado, roadmap por fases con checklists, riesgos, glosario. Cámbialo solo cuando haya un cambio de fondo (nueva fase completada, decisión de arquitectura, hallazgo técnico nuevo) — no en cada sesión.
2. **`references/status.md`** (en esta misma skill) — el estado *actual* del proyecto: en qué fase estamos, qué se decidió recientemente, qué está bloqueado. Es barato de leer y se actualiza con frecuencia. **Leelo siempre al empezar a trabajar en algo no trivial de Voronia.**
3. **`references/architecture.md`** y **`references/conventions.md`** (en esta skill) — resúmenes accionables de arquitectura y convenciones de código, para no tener que releer el plan completo cada vez que hay que escribir código. Se leen cuando el trabajo lo amerita (diseño de un módulo nuevo, escribir código Rust, tocar límites entre crates).

Si el plan maestro y esta skill alguna vez dicen cosas distintas, **el plan maestro gana** (es el documento humano de referencia) — y eso es señal de que hay que actualizar esta skill (ver protocolo al final).

## Límite de contexto y checkpoint de sesión (REGLA CRÍTICA)

El modelo que impulsa esta sesión es `nvidia/z-ai/glm-5.2`. Aunque GLM-5.2 expone nominalmente ~1M de tokens de ventana, el endpoint de NVIDIA limita en la práctica a **≈170 000 tokens**. Operar pegado al techo degrada la calidad, pierde instrucciones y puede truncar la respuesta. Por eso el umbral operativo es **160 000 tokens**: a partir de ahí hay que actuar, no esperar al colapso.

El agente DEBE, en cada interacción:

1. **Monitorear** el uso total de tokens de la sesión.
2. Si el total se acerca a **160 000 tokens** (o la sesión es extremadamente larga y no hay visibilidad clara del contador):
   - **DETENER de inmediato** cualquier generación de código nueva. Lo que ya estaba en curso se termina limpio o se descarta; no se arrancan tareas nuevas.
    - **Crear `agent_state_checkpoint.md`** en la raíz del workspace (`/home/hans/Proyectos/voronia/`). El archivo debe contener, en este orden y en español:
      1. **Objetivo actual** de la sesión (una o dos frases, **incluyendo la fase del roadmap** p.ej. "Fase 1 — parser de .map, problema X").
      2. **Árbol de archivos modificados/creados** en la sesión (rutas relativas al repo, con un `-` delante de las pendientes de commitear).
      3. **Tareas pendientes** (lista con checkboxes, **copia fiel del estado actual de `todowrite`** — ítems, estado `in_progress`/`pending`/`completed`, prioridad — para poder reconstruirla.
      4. **Decisiones de arquitectura tomadas** (las que aún no estén escritas en `references/architecture.md` o en el plan maestro; si ya están, referenciarlas).
      5. **Próximo paso sugerido** para retomar la sesión en una compacidad nueva.
    - **Enviar al usuario el mensaje literal**:
      > `⚠️ Alerta de límite de contexto (160K). Estado del proyecto guardado en agent_state_checkpoint.md. Iniciando proceso de compactación de OpenCode.`
3. **Invocar la compactación del historial** inmediatamente después. La compactación automática ya está habilitada en `opencode.json` (`compaction.auto: true`, `prune: true`, `reserved: 16000`); si por algún motivo no disparó, forzarla vía el comando de compactación de la TUI antes de seguir trabajando.

### Protocolo de reanudación (cuando se retoma una sesión checkpointeada)

Si el usuario pide **"continuar con el trabajo que se dejó"**, "seguir donde lo dejamos", "continuar con la Fase X" (o cualquier frase equivalente, aunque no mencione el checkpoint explícitamente), el agente DEBE, **antes de escribir nada de código nuevo**:

1. **Leer `references/status.md`** para saber en qué fase está el proyecto y qué fue lo último decidido.
2. **Buscar `agent_state_checkpoint.md`** en la raíz del workspace (`/home/hans/Proyectos/voronia/agent_state_checkpoint.md`). Si existe, cargar su contenido completo.
3. Sobre la base de ese checkpoint:
   - Reconstruir el **`todowrite`** con los ítems tal cual figuran en la sección "Tareas pendientes" del checkpoint (mismos textos, mismo orden, mismo estado — `pending`/`in_progress`/`completed` — y mismas prioridades). Si un ítem estaba `completed`, dejarlo `completed`; si estaba `in_progress`, confirmar que sigue siendo el activo.
   - Repasar el **árbol de archivos** del checkpoint: si alguno quedó marcado como pendiente de commitear (`-`), verificar con `git status`/`git diff` que sigue en ese estado antes de seguir tocándolo.
   - Consultar las **decisiones de arquitectura** del checkpoint; si alguna no está reflejada en `references/architecture.md` ni en el plan maestro, considerarla viva y respetarla al continuar.
4. **Confirmar brevemente con el usuario** el punto exacto de reanudación (objetivo + próximo ítem `in_progress`/`pending`), en una o dos líneas.
5. Ejecutar el **próximo paso sugerido** del checkpoint, o —si el usuario redirigió— el que él pida, actualizando el `todowrite` a medida que avanza.

Si **no existe** `agent_state_checkpoint.md` (sesión nueva, sin checkpoint previo), caer al flujo normal: leer `references/status.md`, chequear el roadmap del plan maestro §23, y preguntar a Hans en qué fase/item retomar antes de arrangar cualquier tarea grande.

El `agent_state_checkpoint.md` **no se commitea** salvo decisión explícita de Hans — es un archivo de trabajo efímero para sobrevivir la compactación, no parte del repo. Debería estar cubierto por `.gitignore` (ver `references/conventions.md`). Tras una reanudación exitosa, borrar el `agent_state_checkpoint.md` para que no se confunda con un checkpoint fresco en sesiones posteriores; si se commitea por error, `git rm --cached agent_state_checkpoint.md` y borrar el archivo.

Esta regla **no reemplaza** a la skill global `token-optimization` (minimizar tokens en lectura, exploración y respuestas): la complementa. La optimización reduce la velocidad a la que se llena el contexto; el checkpoint + compactación garantizan que, cuando se llene, no se pierda nada crítico.

### Protocolo de registro de fase al alcanzar límite de contexto (NUEVO — 26 jul 2026)

Cuando el agente detecta que se acerca al umbral operativo de **160 000 tokens** y debe hacer checkpoint + compactación (ver regla crítica arriba), **además de escribir `agent_state_checkpoint.md`**, DEBE:

1. **Identificar la fase actual** (según `references/status.md` y plan maestro §23).
2. **Escribir/actualizar `docs/fase-{N}.md`** (p.ej. `docs/fase-1.md`, `docs/fase-2.md`) con un registro cronológico completo de **todo lo ocurrido en la sesión actual desde el inicio de esa fase**, siguiendo el formato de `docs/fase-0-investigacion.md`:
   - Referencia congelada de Azgaar (versión, commit, .map usado).
   - Cronología de commits (hash, fecha, título, qué hizo).
   - Arquitectura del código producido (módulos, funciones clave, tipos).
   - Algoritmos portados bit-exacto (con snippets críticos: `circumcenter`, `rn`, `Mash`, etc.).
   - Hallazgos críticos y decisiones (divergencias, lone surrogates, placeholders, grid vs pack features, etc.).
   - Inventario de tests (archivo, count, qué valida cada uno).
   - Estado final: checklist fase en plan maestro, working tree limpio, tests/clippy/fmt verdes.
3. **Incluir en el `agent_state_checkpoint.md`** una referencia al archivo de fase actualizado (p.ej. "Fase 1 registrada en `docs/fase-1.md`").
4. Esto garantiza que **tras la compactación no se pierda el hilo narrativo ni técnico** de la fase — el MD de fase sobrevive como fuente de verdad congelada, igual que `fase-0-investigacion.md`.

El archivo `docs/fase-{N}.md` **sí se commitea** (es documentación del proyecto, no efímero). Se actualiza en cada checkpoint de límite de contexto durante esa fase, y se "congela" cuando la fase se cierra (commit final de fase + tilde en plan maestro §23).

## ⚠️ El hallazgo que no se puede olvidar

El `.map`/JSON de Azgaar **no contiene la geometría del mapa** (posiciones de celdas, vecinos, vértices). Azgaar la recalcula cada vez que carga un mapa, a partir de una semilla. Solo se guardan los **atributos por celda** (altura, bioma, cultura, estado, etc.), indexados por número de celda.

Consecuencia directa para cualquier código de importación: para que esos atributos calcen con la geografía correcta, hay que reproducir **bit-exacto** el algoritmo de Azgaar de generación de grilla + Delaunay/Voronoi + repacking (grid → pack), con el mismo PRNG con semilla. Si el generador nativo produce una malla ligeramente distinta, un mapa importado queda con los atributos mal ubicados — sin errores visibles, solo datos incorrectos en silencio. Cualquier trabajo sobre `import`/parser debe validar esto explícitamente contra mapas reales exportados de Azgaar, no solo contra fixtures generados por el propio motor nativo (eso no probaría nada, sería circular).

## Stack tecnológico (resumen — detalle completo en `references/architecture.md`)

| Área | Elección |
|---|---|
| Lenguaje | Rust |
| Render GPU | wgpu |
| Ventana/input | winit |
| UI | egui (`egui-wgpu`, `egui-winit`) |
| Matemática gráfica | glam |
| Triangulación de polígonos | lyon |
| Delaunay/Voronoi | delaunator (a validar bit-exactitud contra Azgaar en investigación) |
| Ruido procedural | noise |
| Grafos/pathfinding | petgraph |
| RNG determinista | rand + rand_pcg (semilla obligatoria en todo lo generativo) |
| Serialización import | serde + serde_json |
| Formato propio `.gmap` | bincode (v1) → evaluar rkyv si hace falta velocidad |
| Texto en GPU | glyphon |
| Paralelismo CPU | rayon |
| Errores | anyhow + thiserror |
| Logging | tracing + tracing-subscriber |

No se pinnean versiones exactas de crates acá — usar las últimas estables al momento de tocar `Cargo.toml`.

## Workspace y límites entre crates (resumen — detalle en `references/architecture.md`)

```
crates/
├── vor-core/     World Data Model puro. Sin lógica, sin render. No depende de nada del resto.
├── vor-import/   Parsers .map/JSON de Azgaar + regeneración de geometría (el hallazgo de arriba).
├── vor-format/   Serialización .gmap.
├── vor-sim/      Motor de simulación procedural.
├── vor-render/   Pipeline wgpu, capas, cámara. NUNCA escribe al World Data Model, solo lee.
├── vor-edit/     Comandos de edición + undo/redo. Los únicos que mutan el World Data Model además de vor-sim.
├── vor-app/      Binario final: winit + egui + orquestación.
└── vor-cli/      Herramientas headless, sin dependencias gráficas.
```

Regla dura: `vor-render` nunca depende de `vor-import`, y nada depende "hacia arriba" de `vor-app`. Si una tarea te está por hacer romper esta regla, es señal de que el dato que necesitás debería vivir en `vor-core`, no que hay que saltarse el límite.

## Convenciones rápidas (detalle completo en `references/conventions.md`)

- **Todo lo generativo es determinista.** Misma semilla + mismos parámetros = mismo resultado, siempre. Si escribís un generador sin semilla explícita, está mal.
- **Structure-of-Arrays** para atributos de celda (`Vec<u8>`, `Vec<u16>`... indexados por id de celda), no array de structs. Mismo patrón que ya usa Azgaar internamente con TypedArrays, por las mismas razones de cache-locality.
- **El render nunca muta el mundo.** Si una función de `vor-render` necesita escribir algo, esa lógica va en `vor-edit`, no ahí.
- Errores: `thiserror` para tipos de error de librería (en cada crate), `anyhow` en los binarios (`vor-app`, `vor-cli`).
- Logging estructurado con `tracing`, no `println!`.
- Tests con semilla fija para todo generador — el mismo seed debe dar output byte-idéntico entre corridas (regresión).
- Identidad de git por defecto: `hanserlodev`, igual que en el resto de proyectos de Hans.

## Créditos y licencia

Voronia es MIT. Cualquier `README`, doc o comentario que hable del origen del proyecto debe dar crédito a Azgaar's Fantasy Map Generator (repo, y los tres artículos de referencia que el propio Azgaar cita: Martin O'Leary, Amit Patel, Scott Turner — están en el plan maestro §29). No se copia código de Azgaar — se reimplementa lógica desde cero — así que no hay obligación legal, pero sí es lo correcto mencionarlo siempre que se hable del proyecto hacia afuera.

---

## Protocolo de mantenimiento de esta skill

Esto es lo que hace que esta skill no quede desactualizada — es la parte más importante de este archivo, no un apéndice opcional.

1. **Actualizá `references/status.md` en la misma sesión** en que pase algo relevante: se cierra un ítem del roadmap, se toma una decisión de arquitectura, se descubre algo en la investigación (Fase 0) que cambia una asunción, se agrega/renombra un crate, se resuelve una de las "decisiones pendientes" del plan maestro. No lo dejes para "después" — después nunca llega y la skill se vuelve mentirosa.
2. **Si algo contradice lo que dice esta skill o `references/architecture.md`/`conventions.md`** (ejemplo: se decide no usar egui y cambiar a otra cosa, se renombra un crate, cambia una convención) — **editá esos archivos directamente**, no lo dejes solo anotado en `status.md`. `status.md` es para "qué está pasando ahora"; `SKILL.md`/`architecture.md`/`conventions.md` son para "cómo funciona el sistema de forma estable". Si divergen, alguien (vos mismo, en la próxima sesión) va a confiar en información vieja.
3. **El roadmap con checkboxes vive en el plan maestro (§23), no lo dupliques acá.** Cuando se completa un ítem, tildalo ahí. Esta skill referencia el estado, no lo reemplaza.
4. **Commiteá los cambios de esta skill junto con el código/decisión que describen.** Es exactamente para esto que la skill vive dentro del repo (`.opencode/skills/voronia-dev/`) y no en el home global de Hans: el historial de git de la skill queda sincronizado con el historial de git del proyecto. Si estás a punto de cerrar una sesión de trabajo con cambios de fondo y no tocaste ningún archivo de esta skill, probablemente te falta actualizar algo.
5. **Si le vas a dar a Hans una recomendación que no está reflejada en ningún lado todavía y la acepta, esa es la señal de escribirla antes de terminar la sesión** — no asumas que se va a acordar de repetírtela la próxima vez.
6. **Registro de fase al límite de contexto (protocolo § "Protocolo de registro de fase al alcanzar límite de contexto")**: cada vez que se dispare el checkpoint por límite de 160K tokens, el agente DEBE escribir/actualizar `docs/fase-{N}.md` con el registro cronológico completo de la fase actual (formato `fase-0-investigacion.md`). Este archivo SÍ se commitea y congela el conocimiento de la fase. Ver sección "Protocolo de registro de fase al alcanzar límite de contexto" arriba para el formato exacto.
