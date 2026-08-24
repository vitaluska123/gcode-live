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

The current flat `src/` layout is in a **transitional state**. The following
modules have already been extracted, but have not yet been moved into their
final folders:

```text
src/
├── app.rs                       current callback wiring and composition
├── app_state.rs                 AppState ownership
├── app_settings.rs              Settings ↔ UiSettings mapping
├── app_file.rs                  editor-safe source G-code preparation
└── app_preview_actions.rs       pan callbacks and cursor-coordinate mapping
```

`app.rs` still owns most file, preview, settings, and export callbacks. The
next refactor must consolidate the transitional `app_*.rs` modules into
`src/app/` and then move the remaining callbacks there. Do not add more
top-level `app_*.rs` modules.

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
