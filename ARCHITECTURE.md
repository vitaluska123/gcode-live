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
use when OpenGL is unavailable. The persisted preview-renderer setting selects
the application-level OpenGL or software backend and applies after restart.

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

## Current Rust layout

The organizational migration is nearly complete:

```text
src/
├── app/
│   ├── mod.rs                   application composition and adapter registration
│   ├── state.rs                 AppState ownership
│   ├── settings.rs              Settings ↔ UiSettings mapping and callbacks
│   ├── file_actions.rs          open/apply source and final G-code callbacks
│   ├── preview_actions.rs       render, zoom, fit, pan and cursor callbacks
│   └── export_actions.rs        export callback and material confirmations
├── domain/
│   ├── frame.rs                 board/frame geometry and tab placement
│   ├── gcode.rs                 temporary public facade for parsing API
│   └── settings.rs              persisted settings model
├── preview/
│   ├── data.rs, input.rs, scene.rs, viewport.rs
│   └── renderer.rs, software_renderer.rs, opengl_renderer.rs
└── export/
    └── gcode.rs                 generated G-code and file writing
```

`app/mod.rs` contains no callback implementations. `app/settings.rs` owns the
UI-settings mapping and settings callbacks. File, preview and export callbacks
are registered only from their respective action modules.

The remaining migration is deliberately small: move the physical G-code parser
implementation (`apply_source_cutting_parameters`, `parse_gcode_*`,
`gcode_words`, and `strip_comments`) from `domain/frame.rs` to
`domain/gcode.rs`. At present, `domain/gcode.rs` is only a facade that
re-exports those functions. Preserve the public API and move the parser tests
with the implementation or import the parser API explicitly from frame tests.

Each migration step must keep the `MainWindow` callback/property contract
unchanged, run `cargo fmt`, `cargo check`, and `cargo test`, and avoid commits
unless explicitly requested by the user.
