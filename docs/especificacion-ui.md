# Voronia UI — Especificación de diseño

> **Nombre del proyecto**: `voronia-ui-v1`  
> **Estado**: Borrador de diseño (no vinculado a una fase del roadmap)  
> **Inspiración**: Azgaar's Fantasy Map Generator (layout funcional, no colores)  
> **Paleta**: Modo oscuro propio de Voronia (no copiar la paleta clara de Azgaar)

---

## 1. Layout general

```
┌──────────────────────────────────────────────────────┐
│  Voronia — <nombre-del-mapa>                         │  ← TopBar
├───────────────────┬──────────────────────────────────┤
│                   │                                  │
│  Layers  Info     │                                  │
│  Tools   Options  │       MAPA (wgpu fullscreen)     │
│  Style   About    │                                  │
│                   │                                  │
│ ────────────────  │                                  │
│ [New] [Export]    │                                  │
│ [Save] [Load]     │                                  │
│ [Reset Zoom]      │                                  │
└───────────────────┴──────────────────────────────────┘
```

### 1.1 TopBar

Una línea horizontal en la parte superior de la ventana. Muestra:

```
Voronia — Sorvik.map                    [FPS: 60] [zoom: 1.2x]
```

- A la izquierda: "Voronia — " + nombre del archivo cargado.
- A la derecha (opcional): métricas de debug (FPS, zoom) en texto pequeño.
- Sin botones de acción (esas van en el StickyFooter).

### 1.2 SidePanel (contenedor principal a la izquierda)

El panel lateral izquierdo tiene dos secciones fijas:

#### a) TabBar (horizontal, arriba del panel)
Seis pestañas, siempre visibles:

| # | Pestaña | Propósito |
|---|---------|-----------|
| 1 | **Layers** | Toggles de capas + presets + orden |
| 2 | **Info** | Inspector de celda + editor de entidad |
| 3 | **Tools** | Editores, Regenerar, Añadir, Mostrar |
| 4 | **Options** | Configuración del mundo y del generador |
| 5 | **Style** | Estilo visual por capa (colores, fuentes, texturas) |
| 6 | **About** | Versión, créditos, licencia, enlaces |

- Solo un tab activo a la vez.
- El tab activo cambia el contenido que se muestra debajo.

#### b) TabContent (rellena el resto del panel)
El contenido de la pestaña activa. Anchura fija ~240px, scroll vertical si el contenido excede el alto.

#### c) StickyFooter (siempre visible al fondo del panel)

```
┌─────────────────────┐
│ [New] [Export] [Save]│
│ [Load] [Reset Zoom]  │
└─────────────────────┘
```

Botones siempre visibles sin importar la pestaña activa. Son acciones globales:

- **New**: Diálogo de nuevo mapa (seed, tamaño, opciones).
- **Export**: Modal con opciones de exportación (PNG, SVG, JSON, .vorn).
- **Save**: Modal de guardado (.vorn a máquina/browser).
- **Load**: Modal de carga (.map / .vorn desde máquina/browser).
- **Reset Zoom**: Vuelve la cámara al encuadre inicial.

---

## 2. Contenido de cada pestaña

### 2.1 Layers

```
┌─────────────────────┐
│ ☑ Heightmap         │
│ ☐ Biomas            │
│ ☑ Ríos              │
│ ☑ Fronteras Estados │
│ ☐ Fronteras Prov.   │
│ ☐ Fronteras Cult.   │
│ ☑ Burgos            │
│ ☐ Labels            │
│                     │
│ Preset: ▾           │
│ [Político]          │
│                     │
│ Orden de capas:     │
│ ┌─────────────────┐ │
│ │ Heightmap    ≡   │ │
│ │ Rivers       ≡   │ │
│ │ Borders      ≡   │ │
│ │ Burgs        ≡   │ │
│ └─────────────────┘ │
└─────────────────────┘
```

- Checkboxes para toggle de capas (heredado de Fase 3).
- Selector de presets: "Político", "Cultural", "Físico", "Personalizado".
- Lista reordenable por drag (para Fase 6: implementar con egui drag).

### 2.2 Info

```
┌─────────────────────┐
│ Celda #452          │
│ Altura: 67          │
│ Bioma: Bosque       │
│ Estado: Tal Empire  │
│ Cultura: Tal        │
│ Provincia: Tal       │
│ Burgo: Tal City      │
│ Río: Río Azul        │
│ Población: 14500 hab │
│                     │
│ ─── Editor ───      │
│ Estado #3           │
│ Nombre: [Tal Em····]│
│ Color:  [#a33·····] │
│ [Aplicar]           │
└─────────────────────┘
```

- Inspector de celda (heredado de Fase 3): muestra info de la celda seleccionada con click derecho.
- Editor de entidad (Fase 5): campos editables para la entidad asociada (State > Burg > Province).
- Si no hay celda seleccionada: muestra "Click derecho en el mapa para seleccionar".

### 2.3 Tools

```
┌─────────────────────┐
│ Edit                │
│  ├ Estados          │
│  ├ Burgos           │
│  ├ Provincias       │
│  ├ Ríos             │
│  └ Culturas         │
│                     │
│ Regenerate          │
│  ├ Estados          │
│  ├ Burgos           │
│  └ Ríos             │
│                     │
│ Add                 │
│  ├ Burgo            │
│  ├ Río              │
│  └ Marker           │
│                     │
│ Show                │
│  ├ Cells (ids)      │
│  ├ Charts           │
│  └ Minimap          │
└─────────────────────┘
```

- Sub-secciones con iconos/separadores.
- Cada entrada abre un editor específico (para Fase 6).
- "Edit → Estados" abre el panel de renombrar/colorear estados (similar al editor actual pero más completo).
- "Regenerate" son atajos a las funciones de `vor-sim`.
- "Show → Minimap" es futura implementación.

### 2.4 Options

```
┌─────────────────────┐
│ Config. del mundo   │
│ (requiere regenerar)│
│                     │
│ Seed:   [12345    ] │
│ Points: [10K ▾    ] │
│ Width:  [1024     ] │
│ Height: [768      ] │
│                     │
│ Heightmap template  │
│ [▾ Seleccionar    ] │
│                     │
│ Cultures: [16     ] │
│ States:   [14     ] │
│ Burgs:    [1000   ] │
│ Religions:[8      ] │
│ Provinces:[200    ] │
│                     │
│ ─── Preferencias -- │
│ Autosave: [☑] cada  │
│ [60] segundos       │
│ Idioma:  [▾ ES    ] │
│                     │
│ [Aplicar y regen.]  │
└─────────────────────┘
```

- Sección "Configuración del mundo": parámetros que requieren regenerar el mapa.
- Sección "Preferencias": ajustes de UI/autosave/idioma que aplican inmediato.
- Botón "Aplicar y regenerar" (Fase 7+ cuando vor-sim pueda regenerar).

### 2.5 Style

```
┌─────────────────────┐
│ Elemento: ▾         │
│ [Borders ▾        ] │
│                     │
│ Color:  [#e63333  ] │
│ Opacidad: [━━━●━━━] │
│ Ancho:   [━━●━━━━] │
│                     │
│ ─── Labels ───      │
│ Font: [▾ Open Sans] │
│ Tamaño: [12      ] │
│ Sombra: [☑]        │
│                     │
│ [Aplicar a capa]    │
└─────────────────────┘
```

- Selector de elemento (Borders, Ocean, Rivers, Labels, States, Provinces, Burgos, etc.)
- Controls de estilo por elemento: color, opacidad, ancho de trazo, fuente, tamaño, sombra.
- Los cambios se aplican en tiempo real al render (Fase 6+).

### 2.6 About

```
┌─────────────────────┐
│ Voronia v0.1.0      │
│ Motor de mapas de   │
│ fantasía nativo     │
│ (Rust + wgpu)       │
│                     │
│ Basado en           │
│ Azgaar's Fantasy    │
│ Map Generator       │
│ (MIT)               │
│                     │
│ Por Hans            │
│ Código abierto      │
│ github.com/...      │
└─────────────────────┘
```

---

## 3. Paleta de colores (modo oscuro Voronia)

No se copia la paleta clara de Azgaar. Se mantiene el modo oscuro actual:

| Elemento | Color | Uso |
|----------|-------|-----|
| Fondo panel | `#1e1e2e` | Fondo del SidePanel |
| Fondo tabs inactivos | `#2a2a3e` | Tabs no seleccionados |
| Fondo tab activo | `#3a3a5e` | Tab seleccionado |
| Texto | `#cdd6f4` | Texto normal |
| Texto secundario | `#6c7086` | Labels, hints |
| Borde | `#45475a` | Separadores, bordes de input |
| Acento | `#89b4fa` | Botones, links, selección |
| Error | `#f38ba8` | Validación fallida |
| Fondo input | `#313244` | Campos de texto |

Basado en Catppuccin Mocha (el tema oscuro que ya usa Voronia implícitamente).

---

## 4. Modales (dialogs)

Export, Save, Load, New Map se abren como modales egui (ventanas flotantes centradas).

### 4.1 Export dialog

```
┌──────────────────────────────┐
│ Exportar mapa                │
├──────────────────────────────┤
│ Formato:                     │
│ [● PNG] [○ SVG] [○ JPEG]    │
│                              │
│ Escala: [━━━●━━━━━━━] 2x    │
│                              │
│ [Exportar y descargar]      │
│                              │
│ ─── Datos ───                │
│ GeoJSON: cells routes rivers │
│ JSON: [▾ full/minimal      ] │
└──────────────────────────────┘
```

### 4.2 Save dialog

```
┌──────────────────────────────┐
│ Guardar mapa                 │
├──────────────────────────────┤
│ Destino:                     │
│ [● Máquina] [○ Browser]      │
│                              │
│ Nombre: [Sorvik.vorn      ]  │
│                              │
│ [Guardar]                    │
└──────────────────────────────┘
```

### 4.3 Load dialog

```
┌──────────────────────────────┐
│ Cargar mapa                  │
├──────────────────────────────┤
│ Origen:                      │
│ [● Máquina] [○ URL]          │
│                              │
│ [Seleccionar archivo...]     │
│                              │
│ Formatos: .map, .vorn       │
└──────────────────────────────┘
```

---

## 5. Criterios de implementación

1. **Cada pestaña es un módulo separado** en `vor-app/src/ui/` para mantener el `lib.rs` manejable.
2. **El TabBar + StickyFooter son estáticos** (no cambian entre tabs).
3. **El estado activo del tab** vive en `State` como `active_tab: TabId` enum.
4. **Los modales** se implementan con `egui::Window` (modal blocking).
5. **Dark mode es el único modo** — no hay toggle claro/oscuro (por ahora).
6. **La info de celda** (tab Info) se sigue poblando desde el picking de click derecho en el mapa.
7. **El editor de entidad** (dentro de Info o Tools) usa `vor-edit` + `EditBuffer` (ya existe).

---

## 6. Tabs futuros (post-Fase 6)

| Tab | Cuando | Por qué |
|-----|--------|---------|
| **History** | Fase 6 | Timeline de undo/redo |
| **Simulation** | Fase 7 | Control de simulación (guerras, economía, clima) |
| **Export → Batch** | Fase 8 | Exportación headless por lotes |

---

## 7. Maquetas de referencia

Para la implementación visual, referirse a:

- **Azgaar**: layout de tabs + sticky footer + modales. Copiar la UX, NO los colores.
- **egui demo**: `egui_demo_app` tiene ejemplos de tabs, collapsing headers, modales.
- **Catppuccin Mocha**: paleta de colores oficial de Voronia (dark mode).
