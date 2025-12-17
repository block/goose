Terminal UI for goose

## Commands

```bash
cargo build -p goose-tui
cargo test -p goose-tui
cargo run -p goose-tui
cargo run -p goose-tui -- --session <ID>
cargo run -p goose-tui -- --recipe <FILE>
cargo run -p goose-tui -- --recipe <FILE> --headless
cargo run -p goose-tui -- --cli
cargo run -p goose-tui -- server --port 3000
```

## Structure

```
src/
├── main.rs           # Entry, CLI args, embedded server setup
├── lib.rs            # Public exports, analysis_target module
├── app.rs            # App orchestrator, component coordination
├── runner.rs         # Event loops (interactive and recipe modes)
├── action_handler.rs # Async side effects (API calls, spawning tasks)
├── tui.rs            # Terminal init/restore (crossterm)
├── headless.rs       # Non-TUI recipe execution
├── cli.rs            # CLI mode (lightweight fallback REPL)
├── at_mention.rs     # @path file attachment processing
├── hidden_blocks.rs  # Strip internal XML blocks from display
├── components/
│   ├── mod.rs        # Component trait definition
│   ├── chat.rs       # Message display, caching, scrolling
│   ├── input.rs      # Text input, slash commands
│   ├── info.rs       # Dynamic status line (spinner, todos, flash messages)
│   ├── status.rs     # Bottom bar (mode, session, tokens, cwd, model, hints)
│   └── popups/
│       ├── mod.rs    # Shared popup utilities
│       ├── help.rs, todo.rs, session.rs, message.rs, config.rs, theme.rs
│       └── builder/  # Custom command creator (multi-file)
├── services/
│   ├── config.rs     # TuiConfig (theme, custom commands)
│   └── events.rs     # EventHandler (crossterm + server events)
├── state/
│   ├── mod.rs        # AppState struct, ActivePopup enum
│   ├── action.rs     # Action enum
│   └── reducer.rs    # State update logic
└── utils/
    ├── styles.rs     # 11 themes
    ├── termimad_renderer.rs  # Markdown → ratatui
    ├── sanitize.rs   # ANSI/control char handling
    ├── spinner.rs    # Spinner animation frames (shared)
    └── layout.rs, json.rs, ascii_art.rs, message_format.rs
```

## Architecture

Elm-like unidirectional data flow:
```
Event → Component.handle_event() → Action → action_handler (side effects) → reducer (state) → render()
```

Key abstractions:
- `Component` trait: `handle_event()` returns `Option<Action>`, `render()` draws to frame
- `AppState`: Central state with `active_popup: ActivePopup` enum for popup visibility
- `Action`: Enum describing state changes
- `action_handler`: Spawns async tasks for API calls, sends results back as Events
- `reducer`: Pure state mutations

TUI embeds goose-server on random port, communicates via `goose-client` crate.

## Rules

Component: Return `Action` from `handle_event()`, never mutate state directly
Component: Check `state.active_popup` in `render()`, return early if not active
State: All mutations go through `reducer::update()`
Async: Side effects go in `action_handler.rs`, spawn tasks that send `Event` back via channel
Cache: ChatComponent caches "sealed" messages, invalidate on width change
Animation: Use frame counter from `Event::Tick`

## Never

Never: Mutate `AppState` outside reducer
Never: Block the event loop with async operations
Never: Put async/side-effect logic in reducer (use action_handler)
Never: Use `unwrap()` on user-provided indices (use `if let Some`)
Never: Forget to restore terminal on panic (tui.rs handles this)

## Entry Points

- Main: `src/main.rs` → `run_tui()` → `runner::run_event_loop()`
- Components: `src/components/mod.rs` (Component trait)
- State: `src/state/mod.rs` (AppState)
- Actions: `src/state/action.rs` (Action enum)
- Side Effects: `src/action_handler.rs`

## Testing

Tests in `tests/` folder.
