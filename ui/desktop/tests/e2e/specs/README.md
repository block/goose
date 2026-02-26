# 🦢🔍 Goose Tester — UI E2E Specs

Business-friendly end-to-end test specifications for the Goose desktop app,
written in [Gherkin](https://cucumber.io/docs/gherkin/) syntax.

These specs are designed to be **executed by Goose itself** using the
`Goose Electron Tester` MCP extension — not by Playwright directly.

## Feature Files

| File | Scenarios | Covers |
|---|---|---|
| `settings.feature` | 3 | Settings page, all tabs, dark mode toggle, Models tab |
| `conversations.feature` | 5 | Start chat, recent chats, load history, follow-ups, new chat |
| `extensions.feature` | 3 | Extensions page, add/remove Running Quotes, use in chat |
| `recipes.feature` | 2 | Recipes page, create recipe from session (chef's hat) |

## How to Run

Ask Goose:

> Run the settings.feature spec

or

> Run all the feature files

Goose will:
1. Read the `.feature` file
2. Read `RUNNER-GUIDE.md` for step definitions and selector mappings
3. Connect to the Electron app via `electron_connect`
4. Execute each scenario step-by-step using Goose Electron Tester MCP tools
5. **Take screenshots as the primary verification method**
6. Report results with pass/fail per scenario

## Verification Philosophy

**Screenshots first.** Most assertions are verified visually via `electron_screenshot`.
DOM queries are only used when a specific computed value is needed (e.g.,
`document.documentElement.className` for theme state). This keeps execution fast
and avoids brittle selector-based assertions.

## Why Gherkin?

- **Industry standard** — understood by QA, PMs, developers
- **Tooling** — syntax highlighting, IDE plugins, linting
- **Extensible** — can later add `playwright-bdd` to run natively too
- **Readable** — business-friendly language, no code knowledge needed

## Key Selector Findings

| Area | data-testid? | Strategy |
|---|---|---|
| Chat elements | ✅ 3 testids | ⚠️ TWO chat-input textareas exist — find visible one (height > 0) |
| Settings tabs | ✅ 7 testids | Direct CSS selector (may need scrollIntoView in narrow windows) |
| Theme buttons | ✅ 3 testids | Direct CSS selector (scroll to element first — below fold) |
| Recipe modal | ✅ 13 testids | Direct CSS selector |
| Extension submit | ✅ 1 testid | scrollIntoView first, avoid clicking near modal edges |
| **Sidebar main nav** | ✅ 6 testids | `nav-home`, `nav-chat`, `nav-recipes`, `nav-scheduler`, `nav-extensions`, `nav-settings` |
| **Start New Chat** | ✅ 1 testid | `nav-start-new-chat` |
| **Show All sessions** | ✅ 1 testid | `nav-show-all-sessions` |
| **Sidebar sessions** | ✅ Dynamic | `sidebar-session-{session_id}` |
| **Recent chats (Home)** | ✅ Dynamic | `recent-chat-{session_id}` |
| **Extension cards** | ✅ Dynamic | `extension-card-{kebab-name}`, `extension-toggle-{name}`, `extension-configure-{name}` |
| **Chef's hat (recipe)** | ✅ 1 testid | `create-recipe-from-session-btn` |
| **Diagnostics (bug)** | ✅ 1 testid | `diagnostics-btn` |
| **Extension modal inputs** | ✅ 4 testids | `extension-name-input`, `extension-description-input`, `extension-command-input`, `extension-endpoint-input` |
| **Extension modal buttons** | ✅ 4 testids | `extension-submit-btn`, `extension-cancel-btn`, `extension-remove-btn`, `extension-confirm-removal-btn` |
| **Confirmation modal** | ✅ 2 testids | `confirmation-cancel-btn`, `confirmation-confirm-btn` |

## Test Run Results

### Run #1 (2026-02-24 morning)
| Feature | Result |
|---|---|
| dark-mode.feature | ✅ 4/4 PASS |
| release-settings.feature | ✅ 3/3 PASS |
| release-conversations.feature | ✅ 4/4 PASS |
| release-extensions.feature | ✅ 2/3 PASS (Scenario 3 blocked by rate limit) |

### Run #2 (2026-02-24 afternoon) — Full Release Checklist
| Feature | Result |
|---|---|
| Settings (3 scenarios) | ✅ 3/3 PASS |
| Conversations (5 scenarios) | ✅ 5/5 PASS |
| Extensions (3 scenarios) | ✅ 2/3 PASS, 1 PARTIAL (tool not found in new session) |
| Recipes (2 scenarios) | ✅ 2/2 PASS |
| **Total** | **12/13 PASS, 1 PARTIAL** |

### Run #3 (2026-02-24 afternoon) — With code fixes
| Feature | Result |
|---|---|
| Settings (3 scenarios) | ✅ 3/3 PASS |
| Conversations (5 scenarios) | ✅ 5/5 PASS |
| Extensions (3 scenarios) | ✅ 3/3 PASS 🎉 (Running Quotes tool invoked!) |
| Recipes (2 scenarios) | ✅ 2/2 PASS (chef's hat via data-testid!) |
| **Total** | **13/13 PASS** 🎉 |

**Code fixes in Run #3:**
- `data-testid="create-recipe-from-session-btn"` on chef's hat button
- `data-testid="diagnostics-btn"` on bug report button
- `data-testid="chat-input-new"` for Hub input (no session)

## Files

```
specs/
├── README.md                      # This file
├── RUNNER-GUIDE.md                # Step definitions, selectors, lessons learned
├── dark-mode.feature              # Theme toggle tests
├── settings-navigation.feature    # Settings tab navigation
├── chat-interaction.feature       # Chat send/receive
├── chat-history.feature           # Message history
├── mcp-extension.feature          # Extension management
├── running-quotes.feature         # MCP tool usage in chat
├── release-settings.feature       # Release: settings verification
├── release-conversations.feature  # Release: conversation flows
├── release-extensions.feature     # Release: extension add/remove/use
└── release-recipes.feature        # Release: recipe navigation/creation
```
