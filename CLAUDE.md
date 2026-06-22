# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```
cargo build            # debug build
cargo build --release  # release build
cargo run -- <file>    # run directly
```

No tests yet. No linter or formatter configured beyond default `cargo clippy` / `cargo fmt`.

## Architecture

argus is a TUI code viewer built with ratatui. Data flows in one direction: `main` → `App` → `Viewer` → `ui::draw`.

- **main.rs** — CLI parsing (clap). Initializes the terminal via `ratatui::init()`, runs the app, then restores the terminal.
- **app.rs** — Event loop. Owns a `Viewer` and dispatches crossterm key events to it. The only place that calls `event::read()`.
- **viewer.rs** — Core state: file content, scroll position, diff state, current hunk index. Exposes scroll/navigation methods and read-only accessors consumed by `ui`. Does not import ratatui widgets directly — delegates all rendering to `ui::draw`.
- **ui.rs** — Stateless rendering. Reads from `Viewer` accessors, builds ratatui widgets, and renders them. Owns the layout split (content area + status bar) and diff line background coloring logic.
- **highlight.rs** — Wraps syntect. `Highlighter::highlight()` takes raw content and returns `Vec<Line<'static>>` with syntax-colored spans. Uses `base16-ocean.dark` theme.
- **diff.rs** — Wraps git2. `DiffState::load()` discovers the repo, computes index-to-workdir diff for the given file, and returns parsed hunks/lines. Returns `Err` if not in a git repo or no diff exists — callers use `.ok()` to degrade gracefully.

Git diff is always auto-detected: `App::new` calls `DiffState::load().ok()`, so the viewer shows diff highlights when changes exist and plain view otherwise.
