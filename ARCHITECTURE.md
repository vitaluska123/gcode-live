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

The Rust layout is in a **transitional state**. The extracted application
modules have been consolidated into `src/app/`:

```text
src/
├── app/
│   ├── mod.rs                   composition and remaining callback wiring
│   ├── state.rs                 AppState ownership
│   ├── settings.rs              Settings ↔ UiSettings mapping and callbacks
│   ├── file_actions.rs          editor-safe source G-code preparation
│   └── preview_actions.rs       pan callbacks and cursor-coordinate mapping
├── exporter.rs                  still flat; to move to export/gcode.rs
├── frame.rs                     still flat; to split into domain/frame.rs and domain/gcode.rs
├── preview.rs                   still flat; to move to preview/data.rs
├── preview_input.rs             still flat; to move to preview/input.rs
├── preview_renderer.rs          still flat; to split under preview/
├── scene.rs                     still flat; to move to preview/scene.rs
├── settings.rs                  still flat; to move to domain/settings.rs
└── viewport.rs                  still flat; to move to preview/viewport.rs
```

`app/settings.rs` owns the UI-settings mapping plus `sync-settings` and
`save-settings` callbacks. `app/mod.rs` still owns callbacks for opening and
applying G-code, preview zoom/fit/cursor/rendering, and TAP export. The next
step is to move those callback groups into `file_actions.rs`,
`preview_actions.rs`, and a new `export_actions.rs`, leaving `app/mod.rs` as
composition only. Do not add top-level `app_*.rs` modules.

The target groups code by responsibility while preserving behaviour and a
single source of state:

```text
src/
├── app/
│   ├── mod.rs                   application composition
│   ├── state.rs                 AppState
│   ├── settings.rs              UiSettings mapping and callbacks
│   ├── file_actions.rs          open/apply source and final G-code
│   ├── preview_actions.rs       zoom, fit, pan, cursor, render callback
│   └── export_actions.rs        export callback and confirmations
├── domain/                      G-code parsing, frame geometry, settings model
├── preview/                     scene, viewport, input, frame data, renderers
└── export/                      generated G-code and file writing
```

Each migration step must keep the `MainWindow` callback/property contract
unchanged, run `cargo fmt`, `cargo check`, and `cargo test`, and avoid commits
unless explicitly requested by the user.
