# txmerge

A TUI diff and merge tool written in Rust. Inspired by WinMerge/WinXMerge, providing side-by-side file comparison and 3-way merge in the terminal.

## Features

- **2-way diff** - Side-by-side comparison with word-level highlighting
- **3-way merge** - Left / Base / Right panel layout with conflict detection
- **File browser dialog** - Browse directories and select files visually (no path typing)
- **Save dialog** - Choose save location with directory browser and filename input
- **WinMerge-compatible shortcuts** - F7/F8 navigation, Alt+Arrow copy operations
- **Mouse support** - Clickable toolbar icons, file browser click selection, dialog close buttons
- **Copy operations** - Merge changes between files with undo/redo support
- **Diff options** - Ignore whitespace, ignore case
- **Status messages** - Visual feedback for save, refresh, and other operations

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

## Toolbar Icons

```
📄 📂 💾 🔄 │ |< < > >| │ -> <- │ ->| |<- │ =>> <<= │ ws Aa
```

| Icon | Action |
|------|--------|
| 📄 | New (planned) |
| 📂 | Open files |
| 💾 | Save files |
| 🔄 | Refresh comparison |
| `|<` `<` `>` `>|` | First / Prev / Next / Last diff |
| `->` `<-` | Copy left-to-right / right-to-left |
| `->|` `|<-` | Copy and advance to next diff |
| `=>>` `<<=` | Copy all |
| `ws` `Aa` | Toggle whitespace / case ignore |

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
| `o` / `Ctrl+O` | Open files (file browser dialog) |
| `Ctrl+S` | Save files (save dialog) |
| `F5` / `Ctrl+R` | Refresh comparison |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+Q` | Quit |

### File Browser Dialog

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate entries |
| `Enter` | Open directory / Select file |
| `Backspace` | Go to parent directory |
| `PageUp` / `PageDown` | Scroll by page |
| `Esc` | Cancel |
| Click | Select entry, click again to open/select |

### Save Dialog

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate directories |
| `Tab` | Enter directory / Copy filename |
| Type | Input filename |
| `Enter` | Save to current directory with input filename |
| `Esc` | Cancel |

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
