# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

txmerge — TUI diff and merge tool written in Rust. Inspired by WinMerge/WinXMerge, providing side-by-side file comparison in the terminal using ratatui.

## Architecture

```
src/
├── main.rs          # Entry point, CLI args (clap)
├── app.rs           # App state (paths, diff result, scroll, mode)
├── events.rs        # Key event handling (crossterm)
├── ui/
│   ├── mod.rs       # Layout: menu bar + diff view + status bar
│   ├── menu_bar.rs  # Unicode icon toolbar
│   ├── diff_view.rs # Side-by-side diff rendering with word-level highlights
│   └── status_bar.rs
├── diff/
│   ├── mod.rs
│   └── engine.rs    # Diff computation using `similar` crate (Myers/Patience)
└── models/
    └── diff_line.rs # DiffLine, DiffResult, LineStatus, WordDiffSegment
```

## Development

```bash
cargo build              # Build
cargo test               # Run tests (diff engine unit tests)
cargo run -- <left> <right>  # Compare two files
cargo run                # Start with blank screen, press 'o' to open files
```

## Key Bindings

- `o` — Open files (enter left/right paths)
- `n` / `F8` — Next diff
- `p` / `F7` — Previous diff
- `Ctrl+Home` / `Ctrl+End` — First / Last diff
- `Alt+→` / `Alt+←` — Copy left→right / right→left
- `Ctrl+→` / `Ctrl+←` — Copy and advance to next
- `F5` — Refresh comparison
- `F9` — Toggle whitespace ignore
- `Ctrl+I` — Toggle case ignore
- `j/k` or `↑/↓` — Scroll
- `q` / `Esc` — Quit

## Workflow Best Practices

- Keep CLAUDE.md under 200 lines per file for reliable adherence
- Perform manual `/compact` at ~50% context usage
- Start with plan mode for complex tasks
- Use human-gated task list workflow for multi-step tasks
- Break subtasks small enough to complete in under 50% context
