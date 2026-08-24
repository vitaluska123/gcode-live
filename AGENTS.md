# AGENTS.md

## Project Overview

This is a desktop application written in Rust and Slint for working with G-code and preparing CNC toolpaths.

When modifying the project, preserve the existing architecture and separation of responsibilities. Do not rewrite large areas of the project unless the task genuinely requires it.

Before changing code, inspect the related modules and reuse existing types, functions, and patterns whenever practical.

## Task Workflow

Follow this workflow for every task:

1. Analyze the request and the relevant codebase before making changes.
2. Identify the affected modules, the appropriate home for each responsibility, and reusable existing structures.
3. If information needed for a correct implementation is missing or ambiguous, compile **all** necessary clarifying questions and ask them together in one message. Do not ask questions one at a time when they can be anticipated up front.
4. Once sufficient information is available, create a concise, actionable implementation plan.
5. Execute the plan immediately. Do not wait for additional confirmation unless the task requires new authority, introduces material risk, or changes scope.
6. Validate the implementation and report the outcome.

For straightforward tasks, state the analysis and plan concisely, then proceed without unnecessary questions.

## Architecture

Keep the following concerns separate:

- UI;
- application state;
- business logic;
- geometry;
- rendering;
- user input;
- file operations.

The UI must not contain business logic. Slint files are responsible for presenting the interface and forwarding events to Rust.

The application composition module (`app.rs` or `app/mod.rs`) must only compose and connect components. Do not put complex calculations, parsing, geometry, rendering logic, or large callback implementations in it.

## Object Model

Prefer cohesive structs with methods over collections of unrelated functions and global state.

Good:

```rust
struct Viewport {
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
}

impl Viewport {
    fn pan_by(&mut self, dx: f64, dy: f64) { /* ... */ }

    fn zoom_at(&mut self, x: f64, y: f64, factor: f64) { /* ... */ }

    fn screen_to_world(&self, /* ... */) -> Point { /* ... */ }
}
```

Avoid related free functions with many shared parameters, such as `calculate_pan`, `calculate_zoom`, and `convert_mouse_coordinates`.

If several values are always used together, model them with a dedicated type.

## No Hard-Coding

Do not place magic numbers in business logic.

Avoid:

```rust
if distance < 20.0 {
    // ...
}
```

Prefer:

```rust
const HIT_TEST_TOLERANCE_PX: f64 = 20.0;
```

or a suitable configuration field.

Keep colors, UI dimensions, and display parameters centralized.

## Function and Module Size

A function should perform one clear task. Split it when it is roughly longer than 40–60 lines, deeply nested, or performs multiple independent actions. Do not fragment simple code artificially.

Do not let a single file become an oversized module. When a file grows beyond roughly 400–600 lines and contains distinct responsibilities, split it accordingly. In particular, do not grow `app.rs` when a separate module is appropriate.

## State Management

Do not introduce new global state.

Avoid many separate `Rc<RefCell<...>>` values when they logically belong to one object. Prefer a cohesive state type:

```rust
struct AppState {
    scene: Scene,
    viewport: Viewport,
    selection: Selection,
}
```

## Error Handling

Do not use `unwrap()` or `expect()` for failures that can result from:

- user files;
- user input;
- operating-system state;
- drivers;
- the file system.

Handle such errors gracefully and present useful feedback to the user.

`unwrap()` is acceptable only for internal invariants whose violation indicates a programmer error.

## Compatibility and Dependencies

Do not add dependencies or APIs that reduce existing platform support without a clear need.

Before adding a dependency, check:

- why it is needed;
- whether existing dependencies can solve the problem;
- supported Windows versions;
- its effect on project size and complexity.

Do not update the Rust edition, Slint, or other major dependencies without a separate reason.

## Performance and Coordinates

Do not perform expensive calculations on every mouse movement when results can be cached.

Do not recreate geometry during pan or zoom. Store geometry in world coordinates. `Viewport` is responsible only for world-to-screen and screen-to-world conversion. Changing the camera must not mutate scene objects.

## Interactive Preview

The preview must use the scene object model:

```rust
struct Scene {
    objects: Vec<SceneObject>,
}

enum SceneObject {
    Toolpath(Toolpath),
    Frame(FrameObject),
    Material(MaterialObject),
}
```

Keep interactivity separate from the renderer:

```text
Pointer input
    -> screen_to_world
    -> hit_test
    -> selection
    -> edit object
    -> renderer
```

The renderer must not mutate the data model.

## Hit Testing

Do not identify a selected object by an image pixel color. Select objects through their geometry in world coordinates.

Every editable object must have a stable identifier.

## Rendering

The renderer must only render `Scene`. Do not put settings changes, G-code parsing, or geometry edits inside it.

The architecture should permit replacing a software renderer with an OpenGL renderer without changing the data model or UI logic.

## Safe Source Editing

When editing existing source files, avoid fragile text-based replacement of large code blocks.

Do not use Python, PowerShell, shell scripts, or similar tools to perform large `.replace()` operations on source code, especially when the matched text contains localized strings, comments, or other non-ASCII text.

Prefer edits anchored to stable code structure, such as:

- function or method signatures;
- module declarations;
- type or implementation blocks;
- callback names;
- nearby Rust syntax.

When moving or refactoring code:

1. Locate the implementation by its code structure rather than by matching the full textual contents of the block.
2. Determine the complete syntactic boundaries of the function, closure, callback, `impl`, or module being changed.
3. Modify only the required range.
4. Preserve existing string literals, comments, line endings, and file encoding unless changing them is explicitly part of the task.
5. Keep Rust and Slint source files encoded as UTF-8.

If an automated patch or replacement fails to apply:

- reread the current version of the file;
- locate the target again using stable code identifiers;
- create a smaller, localized edit;
- do not repeatedly retry the same failed large text replacement;
- do not work around the failure by rewriting the entire file unless genuinely necessary.

Never leave both the old and new implementations in place after extracting code into another module.

## Changing Existing Code

Before creating a new function:

1. Check whether a similar implementation already exists.
2. Reuse existing types.
3. Do not duplicate algorithms.
4. Do not create a second source of truth for the same data.

When refactoring is necessary, keep it minimal and within the task scope.

## Completion Checklist

Before completing a code-change task:

1. Run `cargo fmt`.
2. Run `cargo check`.
3. Run `cargo clippy` when feasible.
4. Fix errors and warnings introduced by the change.
5. Confirm that existing behavior has not regressed.

Do not leave:

- TODOs in place of the requested implementation;
- temporary stubs;
- commented-out legacy code;
- unused dependencies;
- duplicate implementations.

When code has been moved to another module, verify that the original implementation was actually removed and that only one source of truth remains.

## Implementation Scope

Make the smallest set of changes that correctly completes the task. Do not begin a broad refactor unless it is required to deliver the requested result.
