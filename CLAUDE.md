# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```
cargo build            # debug build
cargo build --release  # release build
cargo run -- <file>    # run directly
cargo test             # run all tests
cargo test <test_name> # run a single test
cargo clippy           # lint
cargo fmt              # format
```

## Development Process

**TDD is mandatory.** Always write a failing test BEFORE implementing a fix or feature.

1. Write a test that reproduces the bug or specifies the expected behavior
2. Run the test, confirm it fails
3. Implement the fix
4. Run the test, confirm it passes
5. Run `cargo test` to confirm no regressions

Never push code without a test that covers the change. "It compiles" or "it looks right" is not evidence of correctness.

## Architecture

argus is a TUI code viewer built with ratatui. Data flows in one direction: `main` → `App` → `Viewer` → `ui::draw`.

- **main.rs** — CLI parsing (clap). Initializes the terminal via `ratatui::init()`, runs the app, then restores the terminal.
- **app.rs** — Event loop and input mode management (Normal / Search). Owns a `Viewer` and dispatches crossterm key events to it.
- **viewer.rs** — Core state: file content, scroll position, diff state, current hunk index, search query/matches. Exposes scroll/navigation/search methods and read-only accessors consumed by `ui`. Does not import ratatui widgets directly — delegates all rendering to `ui::draw`.
- **ui.rs** — Stateless rendering. Reads from `Viewer` accessors, builds ratatui widgets, and renders them. Owns the layout split (content area + status bar), gutter diff markers, search highlight, and the search input prompt.
- **highlight.rs** — Wraps syntect. `Highlighter::highlight()` takes raw content and returns `Vec<Line<'static>>` with syntax-colored spans. Uses `base16-ocean.dark` theme.
- **diff.rs** — Wraps git2. `DiffState::load()` discovers the repo and computes diff for the given file. Without `--rev`, computes HEAD-to-workdir (staged + unstaged). With `--rev`, supports single revision (shows that commit's diff), `from..to` range, and `from..` (from to HEAD). Returns `Err` if not in a git repo or no diff exists — callers use `.ok()` to degrade gracefully.
- **word_diff.rs** — Wraps similar. `compute_word_diff()` takes old/new line text and returns per-span changed/unchanged markers for word-level diff highlighting.

Git diff is always auto-detected: `App::new` calls `DiffState::load().ok()`, so the viewer shows diff highlights when changes exist and plain view otherwise. `n`/`N` keys navigate diff hunks when no search is active, and search matches when a search is active.
