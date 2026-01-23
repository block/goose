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
