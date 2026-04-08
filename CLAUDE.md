# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

txmerge — TUI diff and merge tool written in Rust. Inspired by WinMerge/WinXMerge, providing side-by-side file comparison and 3-way merge in the terminal using ratatui.

## Architecture

```
src/
├── main.rs              # Entry point, CLI args (clap)
├── app.rs               # App state (paths, diff, 3-way, undo/redo, save)
├── events.rs            # Key + mouse event handling, MenuAction dispatch
├── ui/
│   ├── mod.rs           # Layout + dialog overlays (open, save confirm)
│   ├── menu_bar.rs      # Unicode icon toolbar with mouse hit-test
│   ├── diff_view.rs     # 2-way side-by-side diff rendering
│   ├── three_way_view.rs # 3-way merge rendering (Left|Base|Right)
│   └── status_bar.rs
├── diff/
│   ├── mod.rs
│   ├── engine.rs        # 2-way diff (similar crate, Myers/Patience)
│   └── three_way.rs     # 3-way merge (base↔left, base↔right hunk merge)
└── models/
    └── diff_line.rs     # DiffLine, DiffResult, ThreeWayLine, ThreeWayResult
```

## Development

```bash
cargo build                                    # Build
cargo test                                     # Run tests (16 tests)
cargo run -- <left> <right>                    # 2-way diff
cargo run -- <left> <base> <right>             # 3-way merge
cargo run                                      # Blank screen, press 'o'
```

## Key Bindings

- `o` — Open files (choose 2-way or 3-way)
- `Ctrl+S` — Save files
- `Ctrl+Z` / `Ctrl+Y` — Undo / Redo
- `n` / `F8` — Next diff
- `p` / `F7` — Previous diff
- `Ctrl+Home` / `Ctrl+End` — First / Last diff
- `Alt+→` / `Alt+←` — Copy left→right / right→left
- `Ctrl+→` / `Ctrl+←` — Copy and advance to next
- `F5` — Refresh comparison
- `F9` — Toggle whitespace ignore
- `Ctrl+I` — Toggle case ignore
- `j/k` or `↑/↓` — Scroll
- `q` / `Esc` — Quit (with save confirmation if unsaved)

## Workflow Best Practices

- Keep CLAUDE.md under 200 lines per file for reliable adherence
- Perform manual `/compact` at ~50% context usage
- Start with plan mode for complex tasks
- Use human-gated task list workflow for multi-step tasks
- Break subtasks small enough to complete in under 50% context
