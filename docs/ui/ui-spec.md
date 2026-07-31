# Voronia UI — Design specification

> **Project name**: `voronia-ui-v1`  
> **Status**: Design draft (not tied to a roadmap phase)  
> **Inspiration**: Azgaar's Fantasy Map Generator (functional layout, not colors)  
> **Palette**: Voronia's own dark mode (do not copy Azgaar's light palette)

---

## 1. General layout

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

A horizontal bar at the top of the window. It shows:

```
Voronia — Sorvik.map                    [FPS: 60] [zoom: 1.2x]
```

- On the left: "Voronia — " + name of the loaded file.
- On the right (optional): debug metrics (FPS, zoom) in small text.
- No action buttons (those go in the StickyFooter).

### 1.2 SidePanel (main left-side container)

The left side panel has two fixed sections:

#### a) TabBar (horizontal, at the top of the panel)
Six tabs, always visible:

| # | Tab | Purpose |
|---|---------|-----------|
| 1 | **Layers** | Layer toggles + presets + order |
| 2 | **Info** | Cell inspector + entity editor |
| 3 | **Tools** | Editors, Regenerate, Add, Show |
| 4 | **Options** | World and generator configuration |
| 5 | **Style** | Per-layer visual style (colors, fonts, textures) |
| 6 | **About** | Version, credits, license, links |

- Only one active tab at a time.
- The active tab changes the content shown below.

#### b) TabContent (fills the rest of the panel)
The active tab's content. Fixed width ~240px, vertical scroll if the content exceeds the height.

#### c) StickyFooter (always visible at the bottom of the panel)

```
┌─────────────────────┐
│ [New] [Export] [Save]│
│ [Load] [Reset Zoom]  │
└─────────────────────┘
```

Buttons always visible regardless of the active tab. They are global actions:

- **New**: New map dialog (seed, size, options).
- **Export**: Modal with export options (PNG, SVG, JSON, .vorn).
- **Save**: Save modal (.vorn to machine/browser).
- **Load**: Load modal (.map / .vorn from machine/browser).
- **Reset Zoom**: Returns the camera to the initial framing.

---

## 2. Content of each tab

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

- Checkboxes to toggle layers (inherited from Phase 3).
- Preset selector: "Político", "Cultural", "Físico", "Personalizado".
- Drag-reorderable list (for Phase 6: implement with egui drag).

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

- Cell inspector (inherited from Phase 3): shows info of the cell selected with right-click.
- Entity editor (Phase 5): editable fields for the associated entity (State > Burg > Province).
- If no cell is selected: shows "Click derecho en el mapa para seleccionar".

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

- Sub-sections with icons/separators.
- Each entry opens a specific editor (for Phase 6).
- "Edit → Estados" opens the panel to rename/color states (similar to the current editor but more complete).
- "Regenerate" are shortcuts to `vor-sim` functions.
- "Show → Minimap" is a future implementation.

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

- "World configuration" section: parameters that require regenerating the map.
- "Preferences" section: UI/autosave/language settings that apply immediately.
- "Aplicar y regenerar" button (Phase 7+ when vor-sim can regenerate).

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

- Element selector (Borders, Ocean, Rivers, Labels, States, Provinces, Burgos, etc.)
- Per-element style controls: color, opacity, stroke width, font, size, shadow.
- Changes apply in real time to the render (Phase 6+).

### 2.6 About

```
┌─────────────────────┐
│ Voronia v0.2.0      │
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

## 3. Color palette (Voronia dark mode)

Azgaar's light palette is not copied. The current dark mode is kept:

| Element | Color | Use |
|----------|-------|-----|
| Panel background | `#1e1e2e` | SidePanel background |
| Inactive tab background | `#2a2a3e` | Unselected tabs |
| Active tab background | `#3a3a5e` | Selected tab |
| Text | `#cdd6f4` | Normal text |
| Secondary text | `#6c7086` | Labels, hints |
| Border | `#45475a` | Separators, input borders |
| Accent | `#89b4fa` | Buttons, links, selection |
| Error | `#f38ba8` | Failed validation |
| Input background | `#313244` | Text fields |

Based on Catppuccin Mocha (the dark theme Voronia already uses implicitly).

---

## 4. Modals (dialogs)

Export, Save, Load, New Map open as egui modals (centered floating windows).

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

## 5. Implementation criteria

1. **Each tab is a separate module** in `vor-app/src/ui/` to keep `lib.rs` manageable.
2. **The TabBar + StickyFooter are static** (they do not change between tabs).
3. **The active tab state** lives in `State` as an `active_tab: TabId` enum.
4. **The modals** are implemented with `egui::Window` (modal blocking).
5. **Dark mode is the only mode** — there is no light/dark toggle (for now).
6. **Cell info** (Info tab) continues to be populated from right-click picking on the map.
7. **The entity editor** (inside Info or Tools) uses `vor-edit` + `EditBuffer` (already exists).

---

## 6. Future tabs (post-Phase 6)

| Tab | When | Why |
|-----|--------|---------|
| **History** | Phase 6 | Undo/redo timeline |
| **Simulation** | Phase 7 | Simulation control (wars, economy, climate) |
| **Export → Batch** | Phase 8 | Headless batch export |

---

## 7. Reference mockups

For the visual implementation, refer to:

- **Azgaar**: tab layout + sticky footer + modals. Copy the UX, NOT the colors.
- **egui demo**: `egui_demo_app` has examples of tabs, collapsing headers, modals.
- **Catppuccin Mocha**: Voronia's official color palette (dark mode).
