# 🦢🔍 Goose Tester — Spec Runner Guide

This document tells Goose how to execute `.feature` (Gherkin) specs against the
Goose desktop Electron app using the **Goose Electron Tester MCP extension**.

## Launching the App

Use `launch-app.sh` to start the app with remote debugging enabled:

### Dev server (with Vite hot reload)
```bash
./launch-app.sh dev /path/to/ui/desktop [port]
# Example:
./launch-app.sh dev /Users/zane/Development/goose/ui/desktop 9224
```
Uses `screen` + `ENABLE_PLAYWRIGHT=1` to launch `npm run start-gui` with a visible Electron window.

### Bundled/packaged app
```bash
./launch-app.sh app "/path/to/Goose.app" [port]
# Example:
./launch-app.sh app "/Users/zane/Downloads/Goose 43.app" 9223
```
Uses `open -a` with `--args --remote-debugging-port=PORT` to launch the app.

### Status & Stop
```bash
./launch-app.sh status 9224    # Check if running
./launch-app.sh stop 9224      # Stop the app
```

## Connecting

After launching, connect via Goose Electron Tester:
```
electron_connect port=9224
```

## Prerequisites

1. The Electron app must be running with remote debugging enabled (use `launch-app.sh`)
2. Connect via `electron_connect` with the port you launched on
3. Verify the app is loaded by taking a screenshot

---

## Verification Philosophy

**Screenshots are the primary verification method.** Use DOM queries only when you
need a specific value (e.g., class name, element count). For visual checks like
"is the response visible?", "does the sidebar show the chat?", "is dark mode active?" —
take a screenshot and verify visually. This keeps test execution fast.

---

## Selector Reference

### Complete data-testid Inventory (from source code audit)

#### Chat & Messages
| Element | Selector | Component |
|---|---|---|
| Chat input (with session) | `[data-testid="chat-input"]` | ChatInput.tsx |
| Chat input (new/hub, no session) | `[data-testid="chat-input-new"]` | ChatInput.tsx (via Hub.tsx) |
| Loading indicator | `[data-testid="loading-indicator"]` | LoadingGoose.tsx |
| Message containers | `[data-testid="message-container"]` | ProgressiveMessageList.tsx |

**NOTE:** The chat input testid is now context-aware:
- `chat-input` — used when there's an active session (in a conversation)
- `chat-input-new` — used on the Hub/new chat page (no session yet)
This fixes the dual-textarea issue where two `chat-input` elements existed simultaneously.

#### Settings Tabs
| Element | Selector | Component |
|---|---|---|
| Models tab | `[data-testid="settings-models-tab"]` | SettingsView.tsx |
| Local Inference tab | `[data-testid="settings-local-inference-tab"]` | SettingsView.tsx |
| Chat tab | `[data-testid="settings-chat-tab"]` | SettingsView.tsx |
| Session tab | `[data-testid="settings-sharing-tab"]` | SettingsView.tsx |
| Prompts tab | `[data-testid="settings-prompts-tab"]` | SettingsView.tsx |
| Keyboard tab | `[data-testid="settings-keyboard-tab"]` | SettingsView.tsx |
| App tab | `[data-testid="settings-app-tab"]` | SettingsView.tsx |

#### Theme
| Element | Selector | Component |
|---|---|---|
| Light button | `[data-testid="light-mode-button"]` | ThemeSelector.tsx |
| Dark button | `[data-testid="dark-mode-button"]` | ThemeSelector.tsx |
| System button | `[data-testid="system-mode-button"]` | ThemeSelector.tsx |

#### Providers
| Element | Selector | Component |
|---|---|---|
| Selection heading | `[data-testid="provider-selection-heading"]` | ProviderSettingsPage.tsx |
| Launch button | `[data-testid="provider-launch-button"]` | CardButtons.tsx |

#### Extensions
| Element | Selector | Component |
|---|---|---|
| Submit button (modal) | `[data-testid="extension-submit-btn"]` | ExtensionModal.tsx |

#### Bottom Bar Actions
| Element | Selector | Component |
|---|---|---|
| Create Recipe from Session (chef's hat) | `[data-session-active="true"] [data-testid="create-recipe-from-session-btn"]` | ChatInput.tsx |
| Diagnostics / Bug Report | `[data-session-active="true"] [data-testid="diagnostics-btn"]` | ChatInput.tsx |

⚠️ **IMPORTANT**: Always scope bottom-bar buttons to `[data-session-active="true"]` because
multiple sessions are mounted simultaneously (hidden via CSS). Without scoping, the selector
picks the first DOM match which may be from a hidden session (zero dimensions, not clickable).

#### Recipes (CreateRecipeFromSessionModal)
| Element | Selector |
|---|---|
| Modal container | `[data-testid="create-recipe-modal"]` |
| Modal header | `[data-testid="modal-header"]` |
| Modal content | `[data-testid="modal-content"]` |
| Modal footer | `[data-testid="modal-footer"]` |
| Create button | `[data-testid="create-recipe-button"]` |
| Create & Run button | `[data-testid="create-and-run-recipe-button"]` |
| Cancel button | `[data-testid="cancel-button"]` |
| Close button | `[data-testid="close-button"]` |
| Form state | `[data-testid="form-state"]` |
| Analyzing state | `[data-testid="analyzing-state"]` |
| Analyzing title | `[data-testid="analyzing-title"]` |
| Analysis stage | `[data-testid="analysis-stage"]` |
| Analysis spinner | `[data-testid="analysis-spinner"]` |

#### Recipe Form Fields
| Element | Selector |
|---|---|
| Form container | `[data-testid="recipe-form"]` |
| Title input | `[data-testid="title-input"]` |
| Description input | `[data-testid="description-input"]` |
| Prompt input | `[data-testid="prompt-input"]` |
| Instructions input | `[data-testid="instructions-input"]` |
| Recipe name input | `[data-testid="recipe-name-input"]` |

#### Misc
| Element | Selector | Component |
|---|---|---|
| Environment badge | `[data-testid="environment-badge"]` | EnvironmentBadge.tsx |
| Geist icon | `[data-testid="geist-icon"]` | icons.tsx |

### Sidebar Navigation (NOW WITH data-testid! ✅)

All main nav items now have `data-testid` attributes:

| Element | Selector | Notes |
|---|---|---|
| Home | `[data-testid="nav-home"]` | |
| Chat | `[data-testid="nav-chat"]` | Toggles chat session list |
| Recipes | `[data-testid="nav-recipes"]` | |
| Scheduler | `[data-testid="nav-scheduler"]` | |
| Extensions | `[data-testid="nav-extensions"]` | |
| Settings | `[data-testid="nav-settings"]` | |
| Start New Chat | `[data-testid="nav-start-new-chat"]` | Inside Chat sub-items |
| Show All | `[data-testid="nav-show-all-sessions"]` | Inside Chat sub-items |

**To click any nav item:**
```
electron_click selector="[data-testid='nav-settings']"
```

#### Sidebar Chat Sessions (NOW WITH data-testid! ✅)

| Element | Selector | Example |
|---|---|---|
| Sidebar session item | `[data-testid="sidebar-session-{session_id}"]` | `sidebar-session-20260224_17` |

**To click a session in the sidebar:**
```
electron_click selector="[data-testid='sidebar-session-20260224_17']"
```

**To find a session by name** (when you don't know the ID):
```javascript
(function() {
  const items = document.querySelectorAll('[data-testid^="sidebar-session-"]');
  for (const item of items) {
    if (item.textContent.includes('CHAT_NAME')) {
      item.click();
      return 'clicked ' + item.getAttribute('data-testid');
    }
  }
  return 'not found';
})()
```

#### Recent Chats on Home Page (NOW WITH data-testid! ✅)

| Element | Selector | Example |
|---|---|---|
| Recent chat item | `[data-testid="recent-chat-{session_id}"]` | `recent-chat-20260224_17` |

**To click a recent chat:**
```
electron_click selector="[data-testid='recent-chat-20260224_17']"
```

**To find a recent chat by name:**
```javascript
(function() {
  const items = document.querySelectorAll('[data-testid^="recent-chat-"]');
  for (const item of items) {
    if (item.textContent.includes('CHAT_NAME')) {
      item.click();
      return 'clicked ' + item.getAttribute('data-testid');
    }
  }
  return 'not found';
})()
```

#### Settings Tabs

`electron_click` with `selector="[data-testid='settings-app-tab']"` works reliably.
No coordinate-based clicking needed.

#### Extension Cards (NOW WITH data-testid! ✅)

Extension names are kebab-cased (e.g., "Running Quotes" → `running-quotes`).

| Element | Selector | Example |
|---|---|---|
| Extension card | `[data-testid="extension-card-{name}"]` | `extension-card-running-quotes` |
| Extension toggle | `[data-testid="extension-toggle-{name}"]` | `extension-toggle-running-quotes` |
| Extension configure (gear) | `[data-testid="extension-configure-{name}"]` | `extension-configure-running-quotes` |

**To click the gear icon on Running Quotes:**
```
electron_click selector="[data-testid='extension-configure-running-quotes']"
```

**To toggle an extension:**
```
electron_click selector="[data-testid='extension-toggle-running-quotes']"
```

#### Extension Modal Elements (NOW WITH data-testid! ✅)

| Element | Selector |
|---|---|
| Extension name input | `[data-testid="extension-name-input"]` |
| Extension description input | `[data-testid="extension-description-input"]` |
| Extension command input (STDIO) | `[data-testid="extension-command-input"]` |
| Extension endpoint input (HTTP/SSE) | `[data-testid="extension-endpoint-input"]` |
| Submit button (Add/Save) | `[data-testid="extension-submit-btn"]` |
| Cancel button | `[data-testid="extension-cancel-btn"]` |
| Remove extension button | `[data-testid="extension-remove-btn"]` |
| Confirm removal button | `[data-testid="extension-confirm-removal-btn"]` |
| Delete cancel button | `[data-testid="extension-delete-cancel-btn"]` |

#### Confirmation Modal (Unsaved Changes, etc.)

| Element | Selector |
|---|---|
| Cancel / "No" button | `[data-testid="confirmation-cancel-btn"]` |
| Confirm / "Yes" button | `[data-testid="confirmation-confirm-btn"]` |

#### Models Tab Content

| Element | How to find |
|---|---|
| Current model name | Visible text in the Models tab content area (e.g., "goose-claude-4-6-opus") |
| Provider name | Text below model name (e.g., "Databricks") |
| "Switch models" button | Text match |
| "Configure providers" button | Text match |
| "Reset Provider and Model" button | Text match (red button) |

#### Bottom Status Bar (left to right)

| Element | Position | How to find | Notes |
|---|---|---|---|
| Working directory | leftmost | Path text, clickable button | Shows current working dir |
| Attachment icon | after dir | Button with 1-path SVG | Paperclip icon |
| Token count | middle | Number like "0.0401" next to token icon | Updates after agent responds |
| Green status dot | before model | Small SVG dot | |
| Model name | center-right | Text "goose-claude-4-6-opus" | |
| Mode indicator | after model | Text "autonomous" | |
| Manage extensions | right | Button titled "manage extensions", shows count "4" | |
| **Chef's hat (Create Recipe from Session)** | **2nd from right** | **Unlabeled button with 2-path SVG at ~x=972** | **Opens "Create Recipe from Session" modal with auto-filled fields** |
| Bug report icon | rightmost | Unlabeled button with 11-path SVG | Opens "Report a Problem" dialog |

**IMPORTANT:** The chef's hat icon for "Create Recipe from Session" is the **2nd button from the right** in the bottom bar. It is NOT the rightmost button (that's bug report). It has no title/aria-label — identify by position or SVG path count (2 paths).

**Note:** The token count tooltip requires a real mouse hover (CDP mouse move),
not just JS `dispatchEvent`. The visible token number in the bar updates after
the agent finishes responding.

---

## Step Definitions

### Navigation Steps

**"Given the app is loaded and the chat input is visible"**
```
1. electron_wait_for selector="[data-testid='chat-input']" timeout=10000 visible=true
2. electron_screenshot to verify
```

**"When I navigate to {page} in the sidebar"**
```
electron_click selector="[data-testid='nav-{page-lowercase}']"
```
Mapping: Home → `nav-home`, Chat → `nav-chat`, Recipes → `nav-recipes`,
Scheduler → `nav-scheduler`, Extensions → `nav-extensions`, Settings → `nav-settings`

**"When I click Start New Chat in the sidebar"**
```
electron_click selector="[data-testid='nav-start-new-chat']"
```

**"When I click on {name} in the recent chats list"**
```
1. electron_evaluate: find text node in main content area (x > 200), get coordinates
2. electron_click at those coordinates
3. electron_screenshot to verify conversation loaded
```

**"When I click the {tab} settings tab"**
```
1. electron_click selector="[data-testid='settings-{tab-lowercase}-tab']"
2. electron_screenshot to verify tab switched
```
Note: Tab name to testid mapping:
- "Models" → `settings-models-tab`
- "Local Inference" → `settings-local-inference-tab`
- "Chat" → `settings-chat-tab`
- "Session" → `settings-sharing-tab` (NOT "session"!)
- "Prompts" → `settings-prompts-tab`
- "Keyboard" → `settings-keyboard-tab`
- "App" → `settings-app-tab`

**"When I scroll {section} into view"**
```
electron_evaluate:
  (function() {
    const els = document.querySelectorAll('h2, h3, h4, p, span, div');
    for (const el of els) {
      if (el.textContent.trim() === '{section}') {
        el.scrollIntoView({ behavior: 'instant', block: 'start' });
        return 'scrolled';
      }
    }
    return 'not found';
  })()
```

### Chat Steps

**"When I type {text} into the chat input and press Enter"** (compound step)

Two selectors depending on context:

**In a session** (after clicking a chat or "Start New Chat"):
```
electron_type selector="[data-session-active='true'] textarea" text="{text}" clear=true press_enter=true
```

**On the Hub/Home page** (typing to start a brand new chat):
```
electron_type selector="[data-testid='chat-input-new']" text="{text}" clear=true press_enter=true
```

**How to know which to use:**
- After `nav-start-new-chat` → session selector (it creates a session immediately)
- After `nav-home` and typing in the home input → `chat-input-new`
- After clicking a sidebar session or recent chat → session selector

**"When I send the message {text}"** (alias for above)
```
Same as above
```

**"Then the agent should respond within {N} seconds"**
```
1. Take screenshot every 5 seconds up to {N} seconds
2. Look for new message-container elements or visible response text
3. Final screenshot to confirm response appeared
```

**"When I wait for the response to complete"**
```
1. Poll for loading-indicator to disappear (may never appear if response is fast)
2. Fallback: poll for message-container count to increase
3. Screenshot to verify
```

### Theme Steps

**"When I click the {theme} theme button"**
```
electron_click selector="[data-testid='{theme-lowercase}-mode-button']"
```

**"Then the page should switch to dark/light mode"**
```
1. electron_evaluate: document.documentElement.className
   // Should be "dark" or "light"
2. electron_screenshot to visually confirm
```

### Extension Steps

**"When I click the Add custom extension button"**
```
electron_evaluate: find button by text, click it, then screenshot to verify modal opened
```

**"When I fill the extension {field} with {value}"**
```
electron_type with appropriate placeholder selector:
- name: input[placeholder="Enter extension name..."]
- description: input[placeholder="Optional description..."]
- command: input[placeholder*="npx"]
```

**"When I click the extension submit button"**
```
electron_click selector="[data-testid='extension-submit-btn']"
```
⚠️ **IMPORTANT**: If you click near the edge of the modal, an "Unsaved Changes"
dialog may appear. If this happens, click "No" to stay in the modal, then retry
the submit click using the data-testid selector.

**"When I remove the {name} extension if it already exists"**
```
1. Check if extension exists: document.body.innerText.includes('{name}')
2. If yes, scroll to it, click gear icon (use coordinate click from evaluate)
3. In the "Update Extension" modal, find "Remove extension" button
   - It may be below the fold — use scrollIntoView or find its coordinates
4. Click "Remove extension"
5. Confirmation dialog appears: "Delete Extension '{name}'"
6. Click "Confirm removal"
7. Screenshot to verify removal
```

**"When I scroll to the {name} extension"**
```
electron_evaluate:
  (function() {
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      if (walker.currentNode.textContent.trim() === '{name}') {
        const parent = walker.currentNode.parentElement;
        const rect = parent.getBoundingClientRect();
        if (rect.x > 200) {
          parent.scrollIntoView({ behavior: 'instant', block: 'center' });
          return 'scrolled';
        }
      }
    }
    return 'not found';
  })()
```

---

## Execution Flow

When Goose receives a command like "run the release-settings.feature spec":

1. **Read** the `.feature` file to understand Scenarios and steps
2. **Read** this RUNNER-GUIDE for step definitions and selector mappings
3. **Connect** to the Electron app via `electron_connect`
4. **Execute** each step using Goose Electron Tester MCP tools
5. **Screenshot** after each significant action — this is the primary verification
6. **Report** results per scenario with pass/fail and screenshots

## Result Format

```
## Feature: {feature name}

### Scenario: {scenario name}
- Step: {step text} ✅
- Step: {step text} ✅
- Step: {step text} ❌ Error: {details}
  [screenshot attached]

Result: FAIL (2/3 steps passed)
```

---

## Tips for Reliable Execution

1. **Screenshots first**: Use screenshots as the primary verification method. Only use DOM queries when you need a specific computed value.
2. **Sidebar nav**: All main nav items have `data-testid` — use `electron_click selector="[data-testid='nav-home']"` etc. "Start New Chat" is `[data-testid="nav-start-new-chat"]`. Chat session names still need TreeWalker text matching.
3. **Recent chats list**: Items in the Home page "Recent chats" section are clickable but need coordinate-based clicking. Find position with TreeWalker (filter x > 200 for main content area), then `electron_click` at those coordinates.
4. **Settings tabs**: `electron_click` with data-testid selector works reliably. Note "Session" tab has testid `settings-sharing-tab` not `settings-session-tab`.
5. **Theme buttons**: `electron_click` with data-testid works reliably. Selected button has class `bg-background-inverse`.
6. **Scrolling**: Use `scrollIntoView({ behavior: 'instant', block: 'start' })` via `electron_evaluate`. The `electron_scroll` tool doesn't work well because the main content has `overflow-hidden`.
7. **Extension modals**: 
   - "Add Extension" submit button: use `[data-testid="extension-submit-btn"]` selector
   - Clicking near modal edges triggers "Unsaved Changes" dialog — click "No" to stay
   - "Remove extension" button may be below the fold — find coordinates first
   - Confirmation dialog has "Confirm removal" button (not just "Remove")
8. **Chat input selectors**: Each session's textarea has `data-testid="chat-input-{sessionId}"`. The active session's wrapper has `data-session-active="true"`. Use `[data-session-active='true'] textarea` for sessions, `[data-testid='chat-input-new']` for Hub. `electron_type` with `press_enter=true` works directly — no native setter workaround needed.
9. **Token count**: Visible in the bottom bar as a number (e.g., "0.0401"). Updates after agent finishes responding. Starts at "0.0000" for new conversations.

### Speed Optimization Tips

1. **Combine type + enter**: Use `electron_type` with `press_enter=true` — one call instead of two.
2. **Skip unnecessary screenshots**: Only screenshot when you need visual verification. Use DOM queries for pass/fail checks.
3. **Batch verifications**: Combine multiple checks in one `electron_evaluate` call.
4. **Reduce waits**: Use `setTimeout` of 8s max for agent responses. For navigation, 1-2s is enough.
5. **Scope to active session**: Always use `[data-session-active="true"]` prefix for session-scoped elements to avoid hitting hidden sessions.
6. **Use testid selectors**: `electron_click selector="[data-testid='nav-settings']"` is faster than JS text matching.
7. **Don't wait for loading indicators**: They appear/disappear too fast. Poll `message-container` count instead.
10. **Loading indicator**: May appear and disappear very quickly, or not at all for fast responses. Don't rely on it — use screenshot verification or message count polling instead.
11. **Conversation naming**: The app auto-names conversations based on content. A "Hello" chat may get renamed to something else after follow-up messages.
12. **App tab may be off-screen**: In narrower windows, the "App" settings tab scrolls off the right edge of the tab bar. Use `tab.scrollIntoView({ inline: 'center' })` before clicking, or use `electron_evaluate` to find and click it.
13. **Theme buttons need scrolling**: After clicking the App tab, the Theme section is below the fold. Scroll to the dark-mode-button element directly: `document.querySelector('[data-testid="dark-mode-button"]').scrollIntoView({ behavior: 'instant', block: 'center' })` before clicking.
14. **Chef's hat icon**: The "Create Recipe from Session" button is the **2nd from right** in the bottom status bar. It has NO title or aria-label. Identify by position (~x=972) or by being the button with a 2-path SVG. The rightmost button (11-path SVG) is the bug report icon.
15. **Extension submit "Unsaved Changes" trap**: The submit button `[data-testid="extension-submit-btn"]` may be near the bottom of the modal. If the click lands near the modal edge, an "Unsaved Changes" dialog appears. Fix: click "No" to stay, then find the button's exact position with `scrollIntoView` + `getBoundingClientRect` and click at those coordinates.

---

## Lessons Learned (from live test runs)

### 2026-02-24: dark-mode.feature — 4/4 PASS

1. `electron_click` with data-testid selectors works great for settings tabs and theme buttons
2. Sidebar nav requires JS text matching — `textContent.trim() === 'Settings'`
3. `scrollIntoView()` via evaluate is the reliable scroll method
4. Theme persistence works across navigation round-trips
5. Selected theme button detected via `bg-background-inverse` class

### 2026-02-24: release-settings.feature — 3/3 PASS

1. All 7 settings tabs verified visible via data-testid query
2. Models tab shows model name, provider, "Switch models" and "Configure providers" buttons
3. Dark/light toggle works via data-testid click

### 2026-02-24: release-conversations.feature — 4/4 PASS

1. **"Start New Chat" is a `<span>` not a `<button>`** — `querySelectorAll('button')` won't find it. Use TreeWalker text node search instead.
2. **Recent chats list items need coordinate clicking** — find position with TreeWalker (filter `rect.x > 200` to ensure main content area, not sidebar), then `electron_click` at coordinates.
3. **Token count starts at "0.0000"** and updates after agent responds (showed "0.0401" after a 2+2 exchange).
4. **Conversations auto-rename** — "Hello" chat became "Basic math question" after a follow-up about 2+2.
5. **New chat shows "Popular chat topics"** — includes starter prompts like "Develop a tamagotchi game".
6. **Loading indicator may not appear** — for fast responses it comes and goes before we can catch it. Use screenshot verification instead.

### 2026-02-24: release-extensions.feature — 2/3 PASS (Scenario 3 blocked by rate limit)

1. **Extensions page shows Default Extensions count** — went from (4) to (5) after adding Running Quotes.
2. **Extension removal flow**: gear icon → "Update Extension" modal → scroll to "Remove extension" → confirmation dialog "Delete Extension 'Running Quotes'" → "Confirm removal".
3. **"Unsaved Changes" dialog trap**: Clicking near the modal edge (e.g., at y=681 which is near the bottom) can trigger an "Unsaved Changes" confirmation. Click "No" to stay, then use `[data-testid="extension-submit-btn"]` for the submit click.
4. **Extension gear icon**: Found by navigating from the extension name text up to the card container, then finding a non-switch button with an SVG child. Use coordinate clicking.
5. **Extension command input placeholder**: `input[placeholder*="npx"]` matches `e.g. npx -y @modelcontextprotocol/my-extension <filepath>`.
6. **After adding extension, it appears in Default Extensions section** with toggle enabled.

### 2026-02-24: Release Checklist Run #2 — 12/13 PASS, 1 PARTIAL

**New findings:**

1. **Dual chat-input textarea**: TWO `[data-testid="chat-input"]` exist in pair view — one hidden (0x0), one visible. CSS selectors pick the hidden one → "Element is not focusable". Fix: iterate all, check `getBoundingClientRect().height > 0`, focus the visible one.
2. **React native setter required**: `el.value = 'text'` doesn't trigger React state. Must use `Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set` + `dispatchEvent(new Event('input', { bubbles: true }))`.
3. **App tab off-screen in narrow windows**: Tab bar scrolls horizontally. Use `tab.scrollIntoView({ inline: 'center' })` before `electron_click`.
4. **Theme buttons below fold on App tab**: After clicking App tab, scroll to dark-mode-button element directly before clicking.
5. **Chef's hat = Create Recipe from Session**: Now has `data-testid="create-recipe-from-session-btn"`.
6. **Bug report icon**: Now has `data-testid="diagnostics-btn"`.
7. **Extension not available in new chat**: After adding Running Quotes to Default Extensions, starting a new chat may not have the tool available immediately. The agent used "Search Available Extensions" instead of `runningQuote` tool.
8. **Reconnection needed**: CDP connection can drop. If `electron_press_key` fails with "Not attached", call `electron_connect` again.
9. **Promise-based waits**: `new Promise(r => setTimeout(r, 5000)).then(() => ...)` works for waiting, but CDP has a 10s timeout. Keep waits under 8s or use polling.

### 2026-02-24: Release Checklist Run #3 — 13/13 PASS 🎉

**Code fixes applied:**
1. Added `data-testid="create-recipe-from-session-btn"` to chef's hat button — one-click recipe test
2. Added `data-testid="diagnostics-btn"` to bug report button — no more confusion with chef's hat
3. Added `data-testid="chat-input-new"` for Hub/new chat input (sessionId=null) — differentiates from session input

**Key improvements over Run #2:**
1. **S3.3 Running Quotes now PASSES** — tool invocation worked, got Emil Zatopek quote
2. **Chef's hat click is now reliable** — `[data-testid="create-recipe-from-session-btn"]` instead of position guessing
3. **Chat input typing pattern**: For pair view, use `querySelectorAll('textarea')`, filter `height > 0`, focus + native setter + Enter. For Hub, use `[data-testid="chat-input-new"]`.
4. **App tab**: Use `scrollIntoView({ inline: 'center' })` then `electron_click` with selector — JS `.click()` alone doesn't trigger React state change
