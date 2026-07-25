# Voronia — convenciones de código (referencia)

## Determinismo, siempre

Todo generador (heightmap, culturas, estados, nombres, lo que sea) recibe una semilla explícita y ningún estado global oculto de aleatoriedad. Misma semilla + mismos parámetros → mismo output, byte-idéntico, en cualquier corrida. Esto no es una preferencia de estilo: es un requisito funcional (compatibilidad de import con Azgaar, tests de regresión, reproducibilidad de bugs). Si estás escribiendo algo con `rand::thread_rng()` en vez de un RNG con semilla explícita pasado como parámetro, pará y arreglalo antes de seguir.

## Layout de datos

Structure-of-Arrays para cualquier colección grande indexada por id de celda: `Vec<u8>`, `Vec<u16>`, `Vec<f32>`, etc., no `Vec<Cell>` con un struct gordo por elemento. Es el mismo patrón que usa Azgaar (con `TypedArray`s de JS) y por las mismas razones: localidad de cache al iterar millones de celdas. Los structs "amigables" (`Culture`, `State`, `Burg`, etc.) están bien como están — son colecciones chicas (decenas/cientos de elementos), no miles/millones.

## Límites de crates

Antes de escribir código que cruza de un crate a otro, revisá `references/architecture.md` (sección de límites) o el diagrama del plan maestro §5. Las reglas duras:
- `vor-render` es de solo lectura sobre el World Data Model. Si necesita escribir algo, esa lógica no va ahí.
- `vor-core` no depende de ningún otro crate del workspace — es la base.
- Nada depende "hacia arriba" de `vor-app` (el binario final orquesta, no lo consume nadie).

## Manejo de errores

- Crates de librería (`vor-core`, `vor-import`, `vor-format`, `vor-sim`, `vor-render`, `vor-edit`): tipos de error propios con `thiserror`, específicos por dominio (`ImportError`, `FormatError`, etc.), no `anyhow::Error` genérico en las firmas públicas.
- Binarios (`vor-app`, `vor-cli`): `anyhow` está bien en el punto de entrada y para el manejo de errores de alto nivel.
- Nunca `.unwrap()`/`.expect()` en código que procesa datos externos (archivos importados, input de usuario) — ahí siempre `Result` propagado con contexto útil. `.unwrap()` está bien en tests o en invariantes verdaderamente imposibles de violar (y con un comentario explicando por qué es imposible).

## Logging

`tracing` (no `println!`, no `eprintln!`, salvo en herramientas realmente triviales de un solo uso). Niveles con criterio: `error!` para fallos reales, `warn!` para cosas raras pero no fatales, `info!` para hitos (mapa cargado, mundo generado, archivo guardado), `debug!`/`trace!` para lo que ayuda a diagnosticar durante desarrollo.

## Testing

- Todo generador procedural: al menos un test que fija una semilla y verifica que el output es estable entre corridas (regresión — no necesariamente que el resultado sea "correcto" en algún sentido absoluto, sino que no cambió sin querer).
- Tests de import: contra mapas reales exportados de Azgaar (no solo contra fixtures generados por el propio motor — eso sería circular y no probaría compatibilidad real).
- Benchmarks (`criterion`) para lo que tiene presupuesto de rendimiento explícito en el plan maestro §24 (carga de `.gmap`, FPS de pan/zoom, tiempo de import) — no hace falta benchmarkear todo, solo lo que tiene un target definido.

## Estilo general

- `cargo fmt` y `cargo clippy` limpios antes de cualquier commit — es lo que corre CI, no tiene sentido descubrirlo ahí.
- Doc-comments (`///`) en toda API pública de cada crate — el plan maestro §19 asume que se puede generar `cargo doc` y que eso sirve como referencia real, no aspiracional.
- Nombres de tipos y campos en español no, en inglés sí (es una convención de la comunidad Rust/OSS, y el proyecto apunta a ser público) — pero comentarios, commits y docs pueden ser en español si es más natural para Hans; no hace falta forzar inglés en la prosa.

## Git

- Identidad por defecto: `hanserlodev` (coherente con el resto de los proyectos de Hans — no la cambies sin que te lo pidan explícitamente).
- Una rama por fase del roadmap (`fase-1-parser`, `fase-2-visor`, etc.), como está sugerido en el plan maestro §27.
- El commit que cierra una fase o toma una decisión de arquitectura debería incluir, si corresponde, el update a esta skill (`references/status.md` como mínimo) — ver el protocolo de mantenimiento en `SKILL.md`.
