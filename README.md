# argus

A terminal-based code reading tool with syntax highlighting and git diff navigation.

## Features

- Syntax highlighting based on file extension
- Automatic git diff visualization (changed lines marked in gutter and highlighted)
- Jump between diff hunks with `n` / `N`
- Incremental search with `/`
- Vim-like keybindings

## Installation

```
cargo install --path .
```

## Usage

```
argus <file>
```

If the file has uncommitted git changes (staged or unstaged), diff regions are automatically highlighted.

## Keybindings

| Key | Action |
|---|---|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Space` / `PageDown` / `Ctrl-f` | Page down |
| `PageUp` / `Ctrl-b` | Page up |
| `Ctrl-d` | Half page down |
| `Ctrl-u` | Half page up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `/` | Start search |
| `n` | Next search match / next diff hunk |
| `N` | Previous search match / previous diff hunk |
| `q` / `Esc` | Quit (or cancel search) |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
