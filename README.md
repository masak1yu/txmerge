# txmerge

<img width="1477" height="752" alt="Image" src="https://github.com/user-attachments/assets/06e699e5-bac2-4763-b730-9a11eb0d46dc" />

A TUI diff and merge tool written in Rust. Inspired by WinMerge/WinXMerge, providing side-by-side file comparison and 3-way merge in the terminal.

## Features

- **2-way diff** - Side-by-side comparison with word-level highlighting
- **3-way merge** - Left / Base / Right panel layout with conflict detection
- **Directory comparison** - Browse changed/added/deleted files between two directories
- **Git integration** - Compare branches or commits as a directory listing; open individual files as 2-way diff; use as `git difftool` and `git mergetool`
- **Inline editing** - Click or press `i` to edit any panel directly
- **Tab management** - Multiple comparison tabs with independent state
- **3-way copy** - Left→Base / Right→Base copy operations for merge resolution
- **Select-all mode** - Toggle AllSel to bulk-copy all diffs at once
- **File browser dialog** - Browse directories and select files visually
- **Save dialog** - Choose save location with directory browser and filename input
- **WinMerge-compatible shortcuts** - F7/F8 navigation, Alt+Arrow copy operations
- **Mouse support** - Click to edit, scroll wheel, toolbar, tab switching
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

# Directory comparison
txmerge <left-dir> <right-dir>

# Git branch/commit comparison (shows directory listing first)
txmerge --git HEAD~1 HEAD
txmerge --git feature main
txmerge --git HEAD              # HEAD vs working tree
txmerge --git --repo /path/to/repo HEAD main

# Interactive mode (press 'o' to open files)
txmerge
```

## Git Integration

### As git difftool

Add to `~/.gitconfig`:

```ini
[diff]
    tool = txmerge
[difftool "txmerge"]
    cmd = txmerge "$LOCAL" "$REMOTE"
```

Then use with:

```bash
git difftool HEAD~1 HEAD -- path/to/file.rs
```

For directory-level comparison, use `--dir-diff`:

```bash
git difftool --dir-diff HEAD~1 HEAD
```

This opens txmerge with two temporary directories containing all changed files, letting you browse and open individual diffs from the directory view.

### As git mergetool

Add to `~/.gitconfig`:

```ini
[merge]
    tool = txmerge
[mergetool "txmerge"]
    cmd = txmerge "$LOCAL" "$BASE" "$REMOTE" --output "$MERGED"
    trustExitCode = true
```

Then use with:

```bash
git mergetool
```

txmerge opens a 3-way merge view. Press `Ctrl+S` to save the resolved result and exit with code 0. Quitting without saving exits with code 1 (conflict unresolved).

### Branch comparison with --git

`txmerge --git [ref1] [ref2] [--repo <path>]` runs `git diff --name-status` and displays the results as a directory listing. Press Enter on any file to open it as a 2-way diff in a new tab (files are extracted via `git show`).

## Toolbar

```
New Open Save Ref │ Prev Next │ LtR RtL │ AllSel │ ws Aa
```

In 3-way mode, `LtR` and `RtL` become `LtM` and `RtM` (copy to Base/Middle).

| Button | Action |
|--------|--------|
| `New` | New comparison (opens in new tab) |
| `Open` | Open files |
| `Save` | Save files |
| `Ref` | Refresh comparison |
| `Prev` `Next` | Previous / Next diff |
| `LtR` `RtL` | Copy left→right / right→left (2-way); left→base / right→base (3-way) |
| `AllSel` | Toggle select-all mode — next copy operation applies to all diffs |
| `ws` `Aa` | Toggle whitespace / case ignore |

### Select-all copy

Press `AllSel` (turns green) to enter select-all mode, then press `LtR` or `RtL` to copy all diffs at once. The mode is cleared automatically after the copy.

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
| `→` / `←` | Scroll right / left (horizontal) |
| `PageDown` / `PageUp` | Scroll by page |
| Mouse scroll | Scroll 3 lines per tick |
| `g` / `G` | Go to top / bottom |

### Copy Operations

| Key | Action (2-way) | Action (3-way) |
|-----|----------------|----------------|
| `Alt+Right` | Copy left → right | Copy left → base |
| `Alt+Left` | Copy right → left | Copy right → base |
| `Ctrl+Right` | Copy + advance to next diff | Copy + advance to next diff |
| `Ctrl+Left` | Copy + advance to next diff | Copy + advance to next diff |

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

### Directory Compare Mode

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Navigate file list |
| `→` / `←` | Horizontal scroll |
| `Enter` | Open selected file pair in new tab |
| `Ctrl+T` | New tab |
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
- [chrono](https://github.com/chronotope/chrono) - Timezone-aware timestamp formatting

## License

MIT
