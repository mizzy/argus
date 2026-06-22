# argus

A terminal-based code reading tool with syntax highlighting and git diff navigation.

## Features

- Syntax highlighting based on file extension
- Automatic git diff visualization (added lines highlighted in green, deleted in red)
- Jump between diff hunks with `n` / `N`
- Vim-like keybindings

## Installation

```
cargo install --path .
```

## Usage

```
argus <file>
```

If the file has uncommitted git changes, diff regions are automatically highlighted.

## Keybindings

| Key | Action |
|---|---|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Space` / `PageDown` | Page down |
| `b` / `PageUp` | Page up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `n` | Jump to next diff hunk |
| `N` | Jump to previous diff hunk |
| `q` / `Esc` | Quit |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
