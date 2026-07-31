# Política de Seguridad

## Versiones soportadas

| Versión | Soportada |
| ------- | --------- |
| main    | ✅ |
| Otras ramas / tags | ❌ |

Únicamente la rama `main` recibe parches de seguridad. Los releases previos no tienen mantenimiento de seguridad activo.

## Reportar una vulnerabilidad

**No abras un issue público para vulnerabilidades de seguridad.**

Enviá el reporte por correo a **hans@voronia.dev** (o por el canal que Hans indique en el perfil del repo).

### Qué incluir en el reporte

- Descripción clara de la vulnerabilidad y su impacto.
- Pasos para reproducirla (o un PoC si es posible).
- Versión/commit afectado.
- Si ya tenés una mitigación o parche sugerido, incluíla.

### Qué esperar después de reportar

1. **Confirmación de recepción** en un plazo de 72 horas.
2. **Evaluación y respuesta** (¿es válida?, severidad, plan de parche) en un plazo de 7 días.
3. Coordinación del *disclosure*: trabajamos con un período de embargo razonable antes de publicar el fix y el aviso.

Si la vulnerabilidad es crítica, priorizamos un hotfix sobre `main` antes que cualquier otra cosa.

## Alcance

Este repositorio contiene:

- `vor-core` / `vor-import` / `vor-sim` / `vor-render` / `vor-edit` / `vor-app` / `vor-cli` — el motor Voronia (Rust + wgpu).
- Herramientas de importación de mapas de Azgaar (`.map`/JSON) y el formato `.vorn`.

**Está fuera de alcance** (no se consideran vulnerabilidades): código de terceros ya publicado en crates.io (reportar al maintainer del crate correspondiente), y dependencias desactualizadas sin explotación demostrada.

## Buenas prácticas (recordatorio)

- Todo lo generativo usa semilla explícita: **misma semilla + mismos parámetros = mismo resultado**. Si encontrás un generador sin semilla, es un bug.
- El render **nunca escribe** al World Data Model (`vor-render` solo lee). Cualquier escritura desde render es una violación de arquitectura y un candidato a bug de seguridad/integridad de datos.
- Nunca se commitean secretos ni claves. Los `.env` no existen en este repo.
