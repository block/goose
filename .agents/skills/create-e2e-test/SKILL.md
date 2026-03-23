---
name: create-e2e-test
description: Create replayable e2e tests for the Goose desktop app. Use when the user wants to record, generate, or verify browser-based UI tests that can run in CI without an AI agent.
---

# Create E2E Test

You are an AI agent that creates replayable e2e test scenarios for the Goose desktop app using agent-browser CLI.

## Goal

Given a test scenario in natural language, you will:

1. Explore the app using agent-browser
2. Record a set of deterministic CLI commands as a batch file that can be replayed without an AI agent

**Do NOT read source code to understand the UI.** Do not read `.tsx`, `.ts`, or `.css` files to find elements. Use `snapshot` to discover what is on the page — that is your only method. The one exception: read source code only when you need to add a `data-testid` attribute.

## App Lifecycle

Every time you need a clean app state — whether starting for the first time, retrying during exploration, or verifying a recording — follow these steps:

1. Use the `e2e-app` skill to stop any running instance and start a new one. Note the **session ID** (e.g., `260320-170823`) and **CDP port**.
2. Connect agent-browser to the CDP port with the session ID as the session name:
   ```bash
   pnpm exec agent-browser --session <session-id> connect <port>
   ```

### Agent-browser Session Isolation

agent-browser uses `--session` to isolate browser contexts. This prevents multiple agents or tests from interfering with each other.

- **Agent (exploration + replay)**: always use the current app session ID as the session name (e.g., `--session 260320-170823`). Pass it to **every** agent-browser command and to the replay script via `--browser-session`.
- **In batch JSON**: do **not** include session names — the replay script handles this.
- **CI**: no `--session` flag needed — the replay script defaults to the recording filename (e.g., `settings-dark-mode.batch.json` → `settings-dark-mode`).

All `agent-browser` commands must be run from `ui/desktop` using `pnpm exec agent-browser`.

## Workflow

### Phase 1: Explore and Record

1. Start the app using the App Lifecycle steps above.

2. Walk through the test scenario step by step. For each step:
   - **Snapshot** — run `snapshot` after each action (and once before the first action) since refs are invalidated by DOM changes
   - **Locate** — identify the element's `@eN` ref from the snapshot, then convert to a stable locator using the Element Locating Strategy (see Reference)
   - **Act** — perform the action using the stable locator
   - **Save** — append the working command to the batch file at `ui/desktop/tests/e2e-tests/recordings/<name>.batch.json`

   If you need a clean app state at any point, restart using the App Lifecycle steps, then replay the saved batch file to catch up before continuing.

   Rules:
   - Use `wait --load networkidle` before snapshotting slow pages
   - Check `agent-browser errors` if something seems wrong
   - Never use `@eN` refs in the recording — convert to stable locators immediately

   Example (assuming start app session ID is `260320-170823`):
   ```bash
   # Snapshot
   agent-browser --session 260320-170823 snapshot
   # Output:
   #   - textbox "Chat input" [ref=e2]
   #   - button "Send" [ref=e3]

   # Locate — get test-id for @e2
   agent-browser --session 260320-170823 get attr @e2 data-testid
   # Output: chat-input

   # Act — count is 1, so find testid works
   agent-browser --session 260320-170823 find testid "chat-input" fill "hello"

   # Snapshot again
   agent-browser --session 260320-170823 snapshot

   # Locate — get test-id for @e3
   agent-browser --session 260320-170823 get attr @e3 data-testid
   # Output: send-button
   agent-browser --session 260320-170823 get count "[data-testid='send-button']"
   # Output: 2 — duplicate! scope to active session

   # Act — count > 1, so narrow the selector to target a unique match
   agent-browser --session 260320-170823 click "[data-active-session='true'] [data-testid='send-button']"
   ```

3. Review the test scenario step by step and confirm you have a recorded command for each one. If any steps are missing, go back to step 2.

   Example batch file (`ui/desktop/tests/e2e-tests/recordings/<name>.batch.json`):

   ```json
   [
     ["wait", "[data-testid='chat-input']"],
     ["fill", "[data-active-session='true'] [data-testid='chat-input']", "hello"],
     ["wait", "[data-active-session='true'] [data-testid='send-button']"],
    ["click", "[data-active-session='true'] [data-testid='send-button']"],
     ["wait", "--text", "Response"]
   ]
   ```

   Do **not** include in the batch file: `snapshot`, `get`, `diff`, `console`, `errors`, `open`, `connect`

   **Never** use `wait <ms>` (e.g., `wait 3000`) in the batch file. Always wait for a specific condition:
   - `wait "[data-testid='element']"` — wait for an element to appear
   - `wait --text "some text"` — wait for text to appear
   - `wait --load networkidle` — wait for page to finish loading
   - `wait --url "**/path"` — wait for navigation

### Phase 2: Verify the Recording

1. Add `wait` commands before actions on dynamic elements. During Phase 1, you used stable locators that run immediately and may hit elements that haven't rendered yet. Add a `wait` before any action that targets a dynamic element:

   Before:
   ```bash
   find testid "chat-response" click    # fails — element not yet on page
   ```

   After:
   ```bash
   wait "[data-testid='chat-response']"
   find testid "chat-response" click
   ```

2. Restart the app using the App Lifecycle steps.

3. Replay the recording:
   ```bash
   bash ui/desktop/tests/e2e-tests/scripts/replay.sh recordings/<name>.batch.json --connect <port> --browser-session <session-id>
   ```
   Always pass the current app session ID. Exit code 0 = pass, non-zero = fail.

4. If replay fails, restart the app, explore the failing step using the Phase 1 cycle (snapshot → locate → convert → act) to find the fix, update the recording, and go back to step 2.

### Phase 3: Write the Scenario

After the recording is verified, write (or update) a scenario file at `ui/desktop/tests/e2e-tests/scenarios/<name>.md` (same base name as the recording, e.g., `settings-dark-mode.batch.json` → `settings-dark-mode.md`). This is a human-readable description of what the test does — the intent, not the implementation.

- Describe each step in terms of **user actions and expected outcomes**, not selectors or test IDs
- Keep it concise — one line per step
- The scenario serves as the source of truth for re-recording if the test breaks

Example (`scenarios/settings-dark-mode.md`):
```markdown
# Settings: Dark Mode Toggle

1. Open Settings
2. Navigate to the App tab
3. Verify the app is in light mode
4. Switch to dark mode and verify it applies
5. Switch back to light mode and verify it applies
```

## Reference

### Element Locating Strategy

For each element, find a stable locator using this priority:

1. **Semantic locator (preferred — zero extra calls)**: use the role and name directly from the snapshot (e.g., `button "Send"` → `find role button --name "Send" click`). If you suspect duplicates (common names like "Submit", "Close"), check `get count` first.
   - Count is 1 → use `find role <role> --name "<name>" <action>`
   - Count > 1 → fall back to step 2

2. **Test ID**: `get attr @eN data-testid` → if exists, use `find testid "<id>" <action>`.
   - If inside a session container (multiple sessions are mounted simultaneously with only one visible), always scope to the active session — no `get count` needed:
     ```bash
     click "[data-active-session='true'] [data-testid='<id>']"
     type "[data-active-session='true'] [data-testid='<id>']" "text"
     fill "[data-active-session='true'] [data-testid='<id>']" "text"
     ```
   - If outside a session container, check `get count` — if duplicates exist, use `find first "[data-testid='<id>']" <action>` or `find nth <index> "[data-testid='<id>']" <action>` (0-based index)

3. **Add a data-testid (last resort)**: if neither above works, add a `data-testid` to the source code.
   - Names must be globally unique and unambiguous. Include the parent component or location, the element type, and its purpose (e.g., `bottom-menu-alert-dot` not `alert-dot`, `session-card` not `card`)
   - Only add the `data-testid` attribute — do not change any other source code
   - Note the code change so it can be committed alongside the test

**Never** use `@eN` refs in recorded commands — they are session-specific.

### Assertions

Use `wait` and `is` commands as assertions in the recording:

- `wait --text "Success"` — assert text appears (with timeout)
- `is visible ".error-message"` — assert element is visible
- `wait --url "**/dashboard"` — assert navigation happened

### Tips

- Run `pnpm exec agent-browser --help` or `pnpm exec agent-browser <command> --help` to learn unfamiliar commands
- Start with `wait --load networkidle` after `open` to ensure the page is ready
- Use `wait --text` over `wait <ms>` — it's more resilient to timing variations
- Keep recordings short — one user journey per file
- Name files descriptively: `login-with-email.batch.json`, `send-chat-message.batch.json`
