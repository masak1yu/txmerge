# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

txmerge — TUI diff and merge tool written in Rust. Inspired by WinMerge/WinXMerge, providing side-by-side file comparison and 3-way merge in the terminal using ratatui.

## Architecture

```
src/
├── main.rs              # Entry point, CLI args (clap)
├── app.rs               # App + TabState (per-tab: paths, diff, undo/redo, edit state)
├── events.rs            # Key + mouse event handling, MenuAction dispatch
├── ui/
│   ├── mod.rs           # Layout + dialog overlays (open, save, close-tab confirm)
│   ├── menu_bar.rs      # Text-label toolbar with mouse hit-test
│   ├── tab_bar.rs       # Tab bar rendering + mouse hit-test
│   ├── diff_view.rs     # 2-way side-by-side diff rendering + raw text editing
│   ├── three_way_view.rs # 3-way merge rendering (Left|Base|Right) + raw text editing
│   ├── dir_view.rs      # Directory comparison list rendering
│   └── status_bar.rs
├── diff/
│   ├── mod.rs
│   ├── engine.rs        # 2-way diff (similar crate, Myers/Patience)
│   ├── three_way.rs     # 3-way merge (base↔left, base↔right hunk merge)
│   └── dir_compare.rs  # Directory scanning + classification (Changed/LeftOnly/RightOnly/Equal)
├── file_browser.rs      # File browser dialog (open/save)
└── models/
    └── diff_line.rs     # DiffLine, DiffResult, ThreeWayLine, ThreeWayResult, DirEntry, DirCompareResult
```

## Development

```bash
cargo build                                    # Build
cargo test                                     # Run tests (48 tests)
cargo run -- <left> <right>                    # 2-way diff (files or dirs)
cargo run -- <left> <base> <right>             # 3-way merge
cargo run                                      # Blank screen, click or 'i' to edit
```

## Key Design Patterns

- **TabState**: Per-tab document state (text, diff, undo/redo, edit). App holds Vec<TabState>.
- **Raw text editing**: During editing mode, diff_view and three_way_view render source text directly (not diff-colored lines). Guarantees input is always visible.
- **F5 never destroys**: Refresh recomputes diff from in-memory text when edits exist, never reloads from disk.
- **Edit state reset on recompute**: recompute_diff_inner clears edit_state to prevent stale display_line references.
- **source_line clamping**: enter_edit_mode clamps source_line to panel's actual line count.
- **3-way copy targets Base**: In 3-way mode, Alt+Right = Left→Base, Alt+Left = Right→Base.
- **h_scroll**: Horizontal scroll offset per tab. Applied via `Paragraph::scroll((0, h_scroll))`. Reset on file open.
- **select_all**: Bool per tab. When true, copy_left_to_right/right_to_left copies all diffs instead of current. Cleared after copy.
- **Dir compare**: TabState.is_dir_compare flag. DirCompareResult holds entry list + selected index. Scroll computed per-frame in dir_view::draw. Enter key opens selected entry in new tab as 2-way diff.

## Key Bindings

- `i` / Click — Enter edit mode
- `Ctrl+T` — New tab
- `Ctrl+W` — Close tab
- `Ctrl+PageDown/Up` — Switch tabs
- `Ctrl+S` — Save files
- `Ctrl+Z` / `Ctrl+Y` — Undo / Redo
- `n` / `F8` — Next diff
- `p` / `F7` — Previous diff
- `→` / `←` — Horizontal scroll
- `Alt+→` / `Alt+←` — Copy (2-way: L↔R, 3-way: L→Base / R→Base)
- `Ctrl+→` / `Ctrl+←` — Copy and advance to next
- `F5` — Refresh comparison
- `F9` — Toggle whitespace ignore
- `Ctrl+I` — Toggle case ignore
- `q` / `Ctrl+Q` — Quit

### Dir Compare Mode
- `↑`/`↓` / `j`/`k` — Navigate list
- `Enter` — Open selected file pair in new tab

## Workflow Best Practices

- Keep CLAUDE.md under 200 lines per file for reliable adherence
- Perform manual `/compact` at ~50% context usage
- Start with plan mode for complex tasks
- Use human-gated task list workflow for multi-step tasks
- Break subtasks small enough to complete in under 50% context
