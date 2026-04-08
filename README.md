# txmerge

A TUI diff and merge tool written in Rust. Inspired by WinMerge, providing side-by-side file comparison and 3-way merge in the terminal.

## Features

- **2-way diff** - Side-by-side comparison with word-level highlighting
- **3-way merge** - Left / Base / Right panel layout with conflict detection
- **WinMerge-compatible shortcuts** - F7/F8 navigation, Alt+Arrow copy operations
- **Mouse support** - Clickable toolbar icons, dialog close buttons
- **Copy operations** - Merge changes between files with undo/redo support
- **File save** - Save merged results back to disk (Ctrl+S)
- **Diff options** - Ignore whitespace, ignore case

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# 2-way diff
txmerge <left-file> <right-file>

# 3-way merge
txmerge <left-file> <base-file> <right-file>

# Interactive mode (press 'o' to open files)
txmerge
```

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `n` / `F8` | Next diff |
| `p` / `F7` | Prev diff |
| `Alt+Home` / `Ctrl+Home` | First diff |
| `Alt+End` / `Ctrl+End` | Last diff |
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `PageDown` / `PageUp` | Scroll by page |
| `g` / `G` | Go to top / bottom |

### Copy Operations

| Key | Action |
|-----|--------|
| `Alt+Right` | Copy left to right |
| `Alt+Left` | Copy right to left |
| `Ctrl+Right` | Copy left to right + next |
| `Ctrl+Left` | Copy right to left + next |

### File Operations

| Key | Action |
|-----|--------|
| `o` / `Ctrl+O` | Open files |
| `Ctrl+S` | Save files |
| `F5` / `Ctrl+R` | Refresh comparison |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+Q` | Quit |

### Options

| Key | Action |
|-----|--------|
| `F9` / `Ctrl+W` | Toggle whitespace ignore |
| `Ctrl+I` | Toggle case ignore |

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [similar](https://github.com/mitsuhiko/similar) - Diff algorithm (Myers/Patience)
- [clap](https://github.com/clap-rs/clap) - CLI argument parsing

## License

MIT
