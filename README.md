# txmerge

A TUI diff and merge tool written in Rust. Inspired by WinMerge/WinXMerge, providing side-by-side file comparison and 3-way merge in the terminal.

## Features

- **2-way diff** - Side-by-side comparison with word-level highlighting
- **3-way merge** - Left / Base / Right panel layout with conflict detection
- **Inline editing** - Click or press `i` to edit any panel directly
- **Tab management** - Multiple comparison tabs with independent state
- **3-way copy** - Left→Base / Right→Base copy operations for merge resolution
- **File browser dialog** - Browse directories and select files visually
- **Save dialog** - Choose save location with directory browser and filename input
- **WinMerge-compatible shortcuts** - F7/F8 navigation, Alt+Arrow copy operations
- **Mouse support** - Click to edit, scroll wheel, toolbar icons, tab switching
- **Undo/Redo** - Full undo/redo support including 3-way base text
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

## Toolbar Icons

```
📄 📂 💾 🔄 │ |< < > >| │ -> <- │ ->| |<- │ =>> <<= │ ws Aa
```

| Icon | Action |
|------|--------|
| 📄 | New comparison (opens in new tab) |
| 📂 | Open files |
| 💾 | Save files |
| 🔄 | Refresh comparison |
| `|<` `<` `>` `>|` | First / Prev / Next / Last diff |
| `->` `<-` | Copy left→right (2-way) or left→base / right→base (3-way) |
| `->|` `|<-` | Copy and advance to next diff |
| `=>>` `<<=` | Copy all |
| `ws` `Aa` | Toggle whitespace / case ignore |

## Key Bindings

### Editing

| Key | Action |
|-----|--------|
| Click / `i` | Enter edit mode on panel |
| Type | Insert text at cursor |
| `Backspace` / `Delete` | Delete character |
| `Enter` | New line |
| `Arrow keys` | Move cursor |
| `Home` / `End` | Move to line start / end |
| `Esc` | Exit edit mode |

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
| Mouse scroll | Scroll 3 lines per tick |
| `g` / `G` | Go to top / bottom |

### Copy Operations

| Key | Action (2-way) | Action (3-way) |
|-----|----------------|----------------|
| `Alt+Right` | Copy left → right | Copy left → base |
| `Alt+Left` | Copy right → left | Copy right → base |
| `Ctrl+Right` | Copy + next | Copy + next |
| `Ctrl+Left` | Copy + next | Copy + next |

### Tab Management

| Key | Action |
|-----|--------|
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+PageDown` | Next tab |
| `Ctrl+PageUp` | Previous tab |
| Click tab | Switch to tab |
| Click `[x]` | Close tab |

### File Operations

| Key | Action |
|-----|--------|
| `o` / `Ctrl+O` | Open files (file browser dialog) |
| `Ctrl+S` | Save files (save dialog) |
| `F5` / `Ctrl+R` | Refresh comparison |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+Q` | Quit |

### Options

| Key | Action |
|-----|--------|
| `F9` | Toggle whitespace ignore |
| `Ctrl+I` | Toggle case ignore |

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [similar](https://github.com/mitsuhiko/similar) - Diff algorithm (Myers/Patience)
- [clap](https://github.com/clap-rs/clap) - CLI argument parsing

## License

MIT
