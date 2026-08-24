# Architecture

## Current boundaries

```text
Slint UI components
    ↓ callbacks and properties
app.rs (composition and callback wiring)
    ↓
AppState
    ├── PreviewScene       world-coordinate board, frame, toolpath and rapid path
    ├── Viewport           zoom and pan only
    ├── PreviewInput       pointer interaction state and screen-to-world mapping
    └── Settings           persisted generation and preview settings
    ↓ immutable snapshot
RenderFrame
    ↓
PreviewRenderer backend
    ├── OpenGlPreviewRenderer
    └── SoftwarePreviewRenderer
```

`PreviewScene` is the source of truth for preview geometry. Renderers receive
only `PreviewSceneSnapshot` through `RenderFrame`; they do not mutate scene,
viewport, settings, or Slint UI state.

Slint's winit/FemtoVG backend owns the native OpenGL context and presentation
surface. The project also compiles Slint's software backend, which Slint can
use when OpenGL is unavailable. `CNC_PREVIEW_RENDERER=software` selects the
application-level software preview backend explicitly.

## UI composition

`ui/main_window.slint` defines the public `MainWindow` contract used by Rust
and composes the screen. Visual sections are isolated in `ui/components/`:

```text
ui/
├── main_window.slint       MainWindow contract and page composition
├── globals.slint           UiSettings bridge to Rust
├── styles/theme.slint      shared UI colors and dimensions
└── components/
    ├── app_header.slint
    ├── settings_panel.slint
    ├── file_information.slint
    ├── preview_panel.slint
    └── gcode_editor.slint
```

Slint components present data and forward events. G-code parsing, frame
calculation, persistence, viewport changes, and rendering policy stay in Rust.

## Direction of the next Rust refactor

The current flat `src/` layout is intentionally being migrated incrementally.
The target is to group code by responsibility: application callback adapters,
G-code/frame domain logic, preview interaction and rendering, and file export.
The move must preserve the existing public behaviour and avoid creating a
second state source.
