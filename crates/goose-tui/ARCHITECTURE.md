# Goose TUI Architecture

## Overview

`goose-tui` is a terminal user interface for Goose built with [ratatui](https://ratatui.rs/) and [crossterm](https://github.com/crossterm-rs/crossterm). It provides an interactive chat interface with markdown rendering, theming, session management, and configuration.

The TUI embeds a goose-server instance and communicates with it via HTTP using the goose-client library. This architecture allows the TUI to leverage all server functionality while maintaining a responsive terminal interface.

For environments where the full TUI isn't suitable, a lightweight `--cli` mode provides the same functionality with simple line-by-line output. See `cli.rs`.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         goose-tui                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ EventHandler │  │     App      │  │      AppState        │   │
│  │  (events.rs) │  │   (app.rs)   │  │    (state/mod.rs)    │   │
│  │              │  │              │  │                      │   │
│  │ - Tick       │  │ - Components │  │ - messages           │   │
│  │ - Input      │──│ - Popups     │──│ - session_id         │   │
│  │ - Mouse      │  │ - Routing    │  │ - is_working         │   │
│  │ - Server     │  │              │  │ - active_popup       │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
│         │                 │                     ▲               │
│         │                 │                     │               │
│         ▼                 ▼                     │               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Event Loop (runner.rs)                     │    │
│  │  event → handle_event() → Action → action_handler →     │    │
│  │                                     reducer → State     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                  │
│                              │ spawn tasks                      │
│                              ▼                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              action_handler.rs                          │    │
│  │  - Spawns async tasks for API calls                     │    │
│  │  - Sends results back as Events via channel             │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                  │
│                              ▼                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    goose-client                         │    │
│  │  - reply()  - start_agent()  - get_providers()  etc.    │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                  │
└──────────────────────────────│──────────────────────────────────┘
                               │ HTTP (localhost:random_port)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Embedded goose-server                        │
│  - Agent management                                             │
│  - Session persistence                                          │
│  - Provider/extension configuration                             │
│  - SSE streaming for responses                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Event Processing

```
┌─────────┐    ┌───────────┐    ┌──────────┐    ┌─────────┐    ┌─────────────┐    ┌───────┐
│ Input   │───▶│ Event     │───▶│ Component│───▶│ Action  │───▶│ action_     │───▶│Reducer│
│(keyboard│    │ Handler   │    │handle_   │    │         │    │ handler     │    │update │
│ mouse)  │    │           │    │event()   │    │         │    │(side effects│    │       │
└─────────┘    └───────────┘    └──────────┘    └─────────┘    └─────────────┘    └───┬───┘
                                                                                      │
┌─────────┐    ┌───────────┐    ┌──────────┐    ┌─────────┐                           │
│ Display │◀───│ ratatui   │◀───│Component │◀───│AppState │◀──────────────────────────┘
│         │    │ Terminal  │    │render()  │    │(updated)│
└─────────┘    └───────────┘    └──────────┘    └─────────┘
```

### Server Communication

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ User sends  │     │action_handler│    │ Task calls  │
│ message     │────▶│spawns task  │────▶│ client.     │
│             │     │ with tx     │     │ reply()     │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                               ▼ SSE stream
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Reducer     │     │ Event loop  │     │ Task sends  │
│ updates     │◀────│ receives    │◀────│ Event::     │
│ state       │     │ event       │     │ Server(msg) │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Component Architecture

### Component Trait

```rust
pub trait Component {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>>;
    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState);
}
```

All UI elements implement this trait. Components:
- Receive events and optionally return Actions
- Render themselves given a frame area and current state
- Do not mutate state directly

### Component Hierarchy

```
App (orchestrator)
├── ChatComponent      # Message list with caching
├── InputComponent     # Text input with slash commands
├── InfoComponent      # Dynamic status (spinner, todos, puns, flash messages)
├── StatusComponent    # Bottom bar (mode, copy indicator, session, tokens, cwd, model, hints)
└── Popups (via ActivePopup enum)
    ├── HelpPopup      # Keybinding reference
    ├── TodoPopup      # Task list from todo_write
    ├── SessionPopup   # Session picker
    ├── BuilderPopup   # Custom command creator/manager
    ├── MessagePopup   # Message detail view with copy/fork
    ├── ConfigPopup    # Provider/extension config
    └── ThemePopup     # Theme selector
```

### Event Priority

App.handle_event() processes in order:
1. **System Events**: Server messages, data loaded events, resize
2. **Global Shortcuts**: Ctrl+C, Ctrl+L, Ctrl+T, Ctrl+S
3. **Popups**: If `active_popup != None`, popup consumes the event
4. **Scroll Debounce**: 500ms delay after popup close to prevent accidental scrolls
5. **Input**: Text input handling
6. **Chat**: Scroll and navigation
7. **Tick**: Animation updates

## State Management

### AppState

Central state struct containing:
- `session_id`: Current session identifier
- `messages`: Conversation history
- `token_state`: Token usage tracking
- `is_working`: Whether agent is processing
- `input_mode`: Normal or Editing
- `active_popup`: Which popup is showing (enum, not flags)
- `todos`: Extracted todo items
- `flash_message`: Temporary status message with expiry
- `config`: TuiConfig (theme, custom commands)
- `available_*`: Loaded data (tools, sessions, providers, extensions)
- `active_provider`, `active_model`: Current provider/model
- `copy_mode`: Whether mouse capture is disabled

### Actions

Action variants grouped by category:
- **System**: Tick, Quit, Refresh, Resize
- **Server/Data**: ServerMessage, SessionResumed, SessionsListLoaded, ToolsLoaded, ProvidersLoaded, ExtensionsLoaded, ModelsLoaded, ConfigLoaded, Error, ShowFlash
- **Chat**: SendMessage, Interrupt, ToggleInputMode, ClearChat
- **UI/Popups**: ToggleTodo, ToggleHelp, OpenSessionPicker, OpenConfig, OpenThemePicker, OpenMessageInfo, ClosePopup, StartCommandBuilder
- **Session**: ResumeSession, CreateNewSession, ForkFromMessage
- **Config**: ChangeTheme, UpdateProvider, ToggleExtension
- **Custom Commands**: DeleteCustomCommand, SubmitCommandBuilder
- **Other**: ToggleCopyMode, SetInputEmpty

### Reducer

`reducer::update(state, action)` handles all state mutations via categorized handler functions. The reducer is pure - no side effects.

### Action Handler

`action_handler::handle_action()` handles side effects before reducer runs:
- Spawns async tasks for API calls (reply, resume session, fetch models, etc.)
- Tasks send results back via Event channel
- Returns `true` for Quit action to signal exit

## Rendering Pipeline

### Markdown Rendering

```
Markdown text
     │
     ▼ termimad
ANSI-styled text
     │
     ▼ parse_ansi_line()
Vec<ratatui::Span>
     │
     ▼ ratatui
Terminal cells
```

### Message Caching

ChatComponent maintains a cache for performance:
- "Sealed" messages (not currently streaming) are cached
- Cache key: message index + terminal width
- Invalidated on: width change, session change, fewer messages
- Last message (if streaming) always rendered fresh

### Animation System

Animations driven by `Event::Tick` (100ms interval):
- Frame counter: `self.frame_count.wrapping_add(1)`
- Breathing effect: `0.85 + 0.15 * sin(frame * 0.1)`
- Spinner: 10-frame braille animation
- Pun rotation: Every 90 ticks (9 seconds)

## Configuration

### TuiConfig

Stored in goose global config (`~/.config/goose/config.yaml`):
- `tui_theme`: Theme name (default: "goose")
- `tui_custom_commands`: User-defined slash commands

### Custom Commands

Custom commands support a `{input}` placeholder in arguments. When the command is invoked with trailing text (e.g., `/mycmd some text`), the placeholder is replaced with that text, enabling dynamic arguments.

### Themes

11 built-in themes: gemini, goose, light, dark, midnight, nord, dracula, matrix, tokyonight, solarized, retrowave

## Integration Points

### goose-server

Embedded on startup with random port:
- Environment: `GOOSE_SERVER__SECRET_KEY`, `GOOSE_PORT=0`
- Graceful shutdown via `CancellationToken`
- 2-second timeout on exit

### goose-client

HTTP client for server communication:
- `reply()`: Stream agent responses (SSE)
- `start_agent()`, `resume_agent()`, `start_agent_with_recipe()`: Session management
- `list_sessions()`, `export_session()`, `import_session()`: Session operations
- `get_providers()`, `get_provider_models()`, `update_provider()`: Provider config
- `get_extensions()`, `add_extension()`, `remove_extension()`: Extension management
- `get_tools()`: Tool discovery
- `upsert_config()`, `read_config()`: Global config

### goose (core)

Used directly for:
- `Config::global()`: Configuration access
- `Message`, `Conversation`: Data types
- `get_enabled_extensions()`: Extension loading
- `ModelConfig`: Context limit lookup

## Design Decisions

### Why Embedded Server?

The TUI embeds goose-server rather than connecting to an external one:
- Self-contained: No separate process to manage
- Consistent: Same server code as desktop app
- Portable: Works offline, no network configuration

### Why Elm Architecture?

Unidirectional data flow provides:
- Predictable state changes
- Easy debugging (log actions)
- Clear separation of concerns
- Testable reducers

### Why Separate action_handler?

Side effects are separated from the reducer:
- Reducer stays pure and testable
- Async operations don't block the event loop
- Clear boundary between state changes and I/O

### Why Cache Messages?

Message rendering is expensive (markdown parsing, ANSI conversion):
- Only re-render when necessary
- Cache invalidation is explicit
- Streaming message always fresh

## Known Limitations

1. **EventHandler Lifecycle**: Background task runs until process exit
2. **Single Session**: One active session at a time (switch via /session)
3. **No Undo**: Actions are not reversible
4. **Memory**: All messages kept in memory (no pagination)
5. **String Parameters Only**: Custom command builder only supports string parameter types
