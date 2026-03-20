---
name: test-recording
description: Record replayable browser test scenarios using agent-browser CLI. Use when the user wants to create, record, or generate browser tests that can be replayed in CI without an AI agent.
---

# Test Recording Skill

You are an AI agent that records replayable browser test scenarios using agent-browser CLI.

## Before You Start

1. Run `agent-browser --help` to see all available commands

If testing an Electron app, connect via CDP:

```bash
agent-browser connect <port>
```

## Command Usage

The first time you use any agent-browser command, run `agent-browser <command> --help` to learn its syntax and options.

You can use any agent-browser command during exploration. Before saving the recording, remove commands that are only useful for the agent to observe the page:
`snapshot`, `screenshot`, `errors`, `get`, `eval`, `console`, `diff`

## Goal

Given a test scenario in natural language, you will:

1. Explore the app using agent-browser
2. Record a set of deterministic CLI commands that can be replayed without an AI agent
3. Save an intent file and a batch recording file

## Workflow

### Phase 1: Explore and Record

For each step of the scenario, follow this cycle:

1. **Snapshot** — always run `snapshot -i` before your first action and after any action, as refs are invalidated by DOM changes
2. **Locate** — build a stable locator for the element (see locating strategy below)
3. **Repeat**

Example:

```bash
# 1. Snapshot
agent-browser snapshot -i
# Output:
#   - textbox "Chat input" [ref=e2]
#   - button "Send" [ref=e3]

# 2. Locate — get test-id for @e2
agent-browser get attr @e2 data-testid
# Output: chat-input

# 3. Act — count is 1, so find testid works
agent-browser find testid "chat-input" fill "hello"

# 4. Repeat — snapshot again
agent-browser snapshot -i

# 2. Locate — get test-id for @e3
agent-browser get attr @e3 data-testid
# Output: send-button
agent-browser get count "[data-testid='send-button']"
# Output: 2 — duplicate! scope to active session

# 3. Act — use CSS selector with data-active-session
agent-browser click "[data-active-session='true'] [data-testid='send-button']"
```

Rules:
- Always `snapshot -i` before your first action on a new page
- Always re-snapshot after any action — refs are invalidated by DOM changes
- Use `wait --load networkidle` before snapshotting slow pages
- Check `agent-browser errors` if something seems wrong
- Never use `@eN` refs in the recording — convert to `find` commands immediately

### Element Locating Strategy

For each element, find a stable locator using this priority:

1. **Test ID**: `get attr @eN data-testid` → if exists, check `get count "[data-testid='<id>']"`
   - Count is 1 → use `find testid "<id>" <action>`
   - Count > 1 → the element is likely inside a chat session container (multiple sessions are mounted simultaneously with only one visible). Scope to the active session using CSS selectors:
     ```bash
     click "[data-active-session='true'] [data-testid='<id>']"
     type "[data-active-session='true'] [data-testid='<id>']" "text"
     fill "[data-active-session='true'] [data-testid='<id>']" "text"
     ```
     If the element is not inside a session container, use `find first "[data-testid='<id>']" <action>` or `find nth <index> "[data-testid='<id>']" <action>` (0-based index)

2. **Semantic locator**: use the role and name from the snapshot → check `get count` with a CSS selector that matches the role and name
   - Count is 1 → use `find role <role> --name "<name>" <action>`
   - Count > 1 → not safe, move to next step

3. **Add a data-testid**: if neither above works, add a `data-testid` to the source code.
   - Names must be globally unique and unambiguous. Include the parent component or location, the element type, and its purpose (e.g., `bottom-menu-alert-dot` not `alert-dot`, `session-card` not `card`)
   - Only add the `data-testid` attribute — do not change any other source code
   - Note the code change so it can be committed alongside the test

Rules:
- **Never** use `@eN` refs in recorded commands — they are session-specific

### Phase 2: Verify the Recording

Replay each recorded command to confirm it works. If a `find` fails because the element hasn't appeared yet, add a `wait` before it, then replay again.

Before:
```bash
find testid "chat-response" click    # fails — element not yet on page
```

After:
```bash
wait "[data-testid='chat-response']" # wait for element to appear
find testid "chat-response" click    # now works
```

Repeat until all commands pass. If a command fails for other reasons, investigate and fix the locator.

### Phase 3: Save

Save the batch recording (`<name>.batch.json`):
```json
[
  ["open", "http://localhost:3000"],
  ["wait", "--load", "networkidle"],
  ["type", "[data-active-session='true'] [data-testid='chat-input']", "hello"],
  ["click", "[data-active-session='true'] [data-testid='send-button']"],
  ["wait", "--text", "Response"]
]
```

Do **not** include in recordings: `snapshot`, `get`, `diff`, `console`, `errors`

## Replay

Use the replay script to run a recording:

```bash
bash tests/agent/replay.sh tests/agent/recordings/<name>.batch.json --connect <port>
```

Exit code 0 = pass, non-zero = fail.

## Assertions

Use `wait` and `is` commands as assertions in the recording:

- `wait --text "Success"` — assert text appears (with timeout)
- `is visible ".error-message"` — assert element is visible
- `wait --url "**/dashboard"` — assert navigation happened

## Tips

- If you need more details on a specific command, run `agent-browser <command> --help`
- Start with `wait --load networkidle` after `open` to ensure the page is ready
- Use `wait --text` over `wait <ms>` — it's more resilient to timing variations
- Keep recordings short — one user journey per file
- Name files descriptively: `login-with-email.batch.json`, `send-chat-message.batch.json`
