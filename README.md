# Argus

A pager with syntax highlighting and git diff visualization.

## Features

- Syntax highlighting based on file extension
- Automatic git diff visualization (changed lines marked in gutter and highlighted)
- Word-level diff highlighting for modified lines
- Jump between change groups with `n` / `N`
- Incremental search with `/`
- Vim-like keybindings

## Installation

### From GitHub Releases

Download the latest binary from [Releases](https://github.com/mizzy/argus/releases).

### From source

```
cargo install --path .
```

## Usage

```
argus <file>
argus --rev HEAD~1 <file>
argus --rev HEAD~3..HEAD <file>
argus --rev abc123..def456 <file>
```

If the file has uncommitted git changes (staged or unstaged), diff regions are automatically highlighted.

With `--rev`, you can view diffs from committed changes:

- `--rev HEAD~1` — diff from the previous commit to HEAD
- `--rev HEAD~3..HEAD` — diff across a range of commits
- `--rev abc123..` — from a commit to HEAD
- `--rev abc123..def456` — between two specific commits

## Keybindings

| Key | Action |
|---|---|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Space` / `PageDown` / `Ctrl-f` | Page down |
| `b` / `Backspace` / `PageUp` / `Ctrl-b` | Page up |
| `Ctrl-d` | Half page down |
| `Ctrl-u` | Half page up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `/` | Start search |
| `n` | Next search match / next change group |
| `N` | Previous search match / previous change group |
| `q` / `Esc` | Quit (or cancel search) |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
