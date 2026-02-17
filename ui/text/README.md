# Goose ACP TUI

Terminal TUI client for goose using ACP over HTTP. Supports two modes:

1. **Single Agent Mode** - Interactive chat with one goose agent
2. **Orchestrator Mode** - Manage multiple goose agents working on different tasks in parallel

## Quick Start

Start the ACP server:
```bash
cargo run -p goose-acp --bin goose-acp-server
```

Run the CLI:
```bash
cd ui/text

# Clone the ACP SDK (required dependency)
git clone https://github.com/agentclientprotocol/typescript-sdk.git acp-sdk
cd acp-sdk && npm install && npm run build && cd ..

# Install dependencies and run
npm install

# Single agent mode (default)
npm start

# Orchestrator mode
npm run orchestrator
```

## Orchestrator Mode

The orchestrator allows you to run multiple goose agents simultaneously, each working on a separate task. When working in a git repository, each workstream gets its own git worktree for isolation.

### Features

- **Parallel Workstreams**: Run multiple goose agents on different tasks
- **Git Worktrees**: Automatic branch and worktree creation for each task
- **Live Monitoring**: Watch any workstream's progress in real-time
- **Notifications**: Get alerted when a workstream needs attention
- **Git Integration**: View diffs, status, and commit changes per workstream

### Usage

```bash
# Start orchestrator in current directory
npm run orchestrator

# Start with custom server
npm run orchestrator -- -s http://localhost:8080

# Start in a specific repository
npm run orchestrator -- -r /path/to/repo
```

### Keyboard Controls

**Dashboard View:**
| Key | Action |
|-----|--------|
| `n` | Create new workstream |
| `↑/↓` or `j/k` | Navigate workstreams |
| `Enter` or `f` | Focus on selected workstream |
| `1-9` | Quick focus by number |
| `s` | Stop selected workstream |
| `q` | Quit orchestrator |
| `?` | Show help |

**Focus View:**
| Key | Action |
|-----|--------|
| `b` or `Esc` | Back to dashboard |
| `m` | Send message to workstream |
| `p` | Pause/resume workstream |
| `s` | Stop workstream |
| `d` | Show git diff |
| `g` | Show git status |
| `c` | Commit changes |

### Workstream Statuses

| Status | Icon | Description |
|--------|------|-------------|
| Starting | ◐ | Setting up worktree and agent |
| Running | ● | Agent is actively working |
| Waiting | ◉ | Needs user input/decision |
| Reviewing | ◈ | Work complete, needs review |
| Paused | ◌ | Paused by user |
| Completed | ✓ | Task finished successfully |
| Error | ✗ | Something went wrong |

### Example Workflow

1. Start the orchestrator: `npm run orchestrator`
2. Press `n` to create a new workstream
3. Enter a name like `fix-login-bug`
4. Describe the task: "Fix the login button not working on mobile"
5. The orchestrator creates a git worktree and starts a goose agent
6. Press `1` to focus on the workstream and watch progress
7. Press `d` to see what changes have been made
8. When done, press `c` to commit the changes

## Single Agent Mode

Classic interactive chat mode with a single goose agent.

```bash
# Interactive mode
npm start

# One-shot mode
npm start -- -p "What is 2+2?"
```

## CLI Options

```
-s, --server <url>  Server URL (default: http://127.0.0.1:3284)
-o, --orchestrator  Enable orchestrator mode
-r, --repo <path>   Repository path for orchestrator (default: cwd)
-p, --prompt <text> One-shot mode (single agent only)
-h, --help          Show help
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Goose Orchestrator TUI                    │
├─────────────────────────────────────────────────────────────┤
│  WorkstreamManager                                          │
│  ├── Workstream 1 ─── AcpClient ─── ACP Server ─── Agent   │
│  ├── Workstream 2 ─── AcpClient ─── ACP Server ─── Agent   │
│  └── Workstream N ─── AcpClient ─── ACP Server ─── Agent   │
├─────────────────────────────────────────────────────────────┤
│  GitWorktreeManager                                         │
│  ├── .goose-worktrees/workstream-1/ (branch: goose/ws-1)   │
│  ├── .goose-worktrees/workstream-2/ (branch: goose/ws-2)   │
│  └── .goose-worktrees/workstream-n/ (branch: goose/ws-n)   │
└─────────────────────────────────────────────────────────────┘
```

## API Endpoints

The ACP server exposes:

- `POST /acp/session` - Create a new session
- `POST /acp/session/{id}/message` - Send a message
- `GET /acp/session/{id}/stream` - SSE event stream

## Development

```bash
# Watch mode
npm run dev

# Type check
npx tsc --noEmit

# Build
npm run build
```
