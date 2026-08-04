# In-app terminal (right panel)

**Started:** 2026-08-03
**Branch:** `feat/in-app-terminal` (issue [#10921](https://github.com/aaif-goose/goose/issues/10921))
**Repo:** goose (`AOrobator/goose` fork, PRs to `aaif-goose/goose`)
**Dogfood base:** `develop`

## Skills Loaded

**Required for this task:**
- `worklog` — multi-session plan + session tracking

**Loaded based on task type:**
- Prior session research (Goose desktop layout + OSS harness survey) — no separate skill file; findings summarized under Problem / Surprises

## Problem

Goose Desktop has no interactive in-app terminal. Cursor/VS Code users expect a toggleable shell beside the chat (Cursor: Cmd+J toggles the panel; ``Ctrl/` `` focuses the terminal). Goose today is chat-first: agent shell runs as non-interactive `developer__shell` (`shell -c`, null stdin) with live output only in chat tool cards. Horizontal space is underused; a **right-side** terminal fits the layout better than a Cursor-style bottom dock (which would fight the floating composer).

This is packaging-heavy (native PTY, Electron rebuild, notarization) and spans UI + main-process IPC + shortcuts — too large for a drive-by PR without a living plan.

## Solution

- [x] Confirm product shape (this worklog) — right panel, Cmd+J, **per chat/session PTY**, agent shell stays separate
- [x] Open/Ready upstream issue with acceptance criteria + non-goals ([#10921](https://github.com/aaif-goose/goose/issues/10921) — move to Ready on board manually; token lacks project scope)
- [x] Spike: `node-pty` in Electron main + xterm.js in renderer over preload IPC (keyed by `sessionId` + `tabId`)
- [x] Session binding: terminals belong to a chat session; create lazily; **keep PTYs alive when switching chats**; kill/dispose on session close (and on app quit)
- [x] Multi-tab **in v1**: each session can have multiple terminal tabs (each tab = its own PTY); Cmd+J toggles the panel for the active session
- [x] Layout: resizable right panel on the active chat; **persist open/closed + width per session across app restarts**; also persist tab set metadata as needed
- [x] Cwd: every new tab/PTY starts in the session’s **current** working directory at spawn time
- [x] Shortcuts: Cmd+J — if panel closed → open + focus terminal; if panel open → close + focus chat
- [x] Shell: login `$SHELL`, honor `GOOSE_SHELL` when set
- [x] Packaging: Forge `asarUnpack` + auto-unpack-natives + vite external `node-pty` (dogfood/smoke still needed)
- [ ] Polish: “Open in external Terminal”, reduced-motion / a11y
- [ ] Dogfood on `develop`; carve clean PR off `upstream/main`

## Key Decision

**Right-side interactive PTY panel, one per chat/session (Hyper pattern), not a bottom dock and not reuse of `developer__shell`.**

- Right side: Goose has unused horizontal space; bottom dock collides with the floating composer + edge fade.
- **Per session/chat, multi-tab:** each conversation owns a terminal panel that can hold **multiple tabs** (each tab = own PTY). Switching chats shows that session’s panel/tabs. Background PTYs **stay alive** while other chats are active.
- **Cmd+J:** closed → open panel + focus active terminal tab; open → close panel + focus chat. (Not a three-way focus cycle while open.)
- **Persist UI state across restarts:** per-session panel open/closed + width (+ tab count/ids as needed). PTY *processes* do not survive app quit — on relaunch, if the panel was open, spawn fresh shell(s) for that session (scrollback not restored in v1).
- **Cwd at spawn:** new tabs always start in the session’s current working directory. Already-running shells are not auto-`cd`’d / restarted when DirSwitcher changes (user can `cd` or open a new tab).
- Interactive PTY in **main** + xterm in **renderer** via IPC. Map keyed by `{sessionId, terminalTabId}` (and window id if multi-window).
- Keep agent tool execution separate from the user’s interactive shell for v1.

## Design Source

No Figma source — user confirmed. Layout cue: Cursor/VS Code terminal UX, but docked **right** instead of bottom to match Goose’s floating composer + wide chat canvas.

## Files

**Likely new**
- `ui/desktop/src/main/ptyService.ts` (or similar) — PTY map keyed by `{windowId, sessionId, terminalTabId}`
- `ui/desktop/src/components/terminal/TerminalPanel.tsx` — right dock chrome, resize, tab strip
- `ui/desktop/src/components/terminal/TerminalTabView.tsx` — xterm host for one tab
- Issue + optional design note under docs or PR body

**Likely modified**
- `ui/desktop/src/main.ts` — IPC handlers, menu item; dispose PTYs on session/window close
- `ui/desktop/src/preload.ts` — `contextBridge` terminal APIs (`create`/`attach`/`write`/`resize`/`kill` by sessionId)
- `ui/desktop/src/components/BaseChat.tsx` and/or `ChatSessionsContainer.tsx` — mount panel for active session; hide/show without destroying other sessions’ PTYs
- `ui/desktop/src/components/AppLayout.tsx` — optional shell for right dock width
- `ui/desktop/src/utils/settings.ts` — default shortcut Cmd+J
- `ui/desktop/src/components/settings/.../KeyboardShortcutsSection.tsx`
- `ui/desktop/package.json` / Forge config — `node-pty`, `@xterm/xterm`, `@xterm/addon-fit`, rebuild/unpack

**Do not conflate**
- `crates/goose/.../developer/shell.rs` — leave as agent tool path
- `ToolCallWithResponse.tsx` — keep chat live output as-is for v1

## Milestones

- [x] **M0 — Issue + worklog sign-off** — [#10921](https://github.com/aaif-goose/goose/issues/10921); plan approved
- [x] **M1 — PTY spike** — main `ptyService` + preload IPC + xterm FitAddon
- [x] **M2 — Per-session right panel + tabs** — Cmd+J, multi-tab, persist open/width/tabs, session cwd, background keep-alive
- [x] **M3 — Packaging hooks** — asarUnpack + auto-unpack-natives + vite external (verify in real app run)
- [ ] **M4 — Product polish** — external Terminal action, a11y, theme polish
- [ ] **M5 — PR** — clean branch off `upstream/main`; dogfood on `develop`

### Commits (tentative)

- [x] `feat(desktop): per-session right-side multi-tab terminal (Cmd+J)`
- [ ] `fix(desktop): terminal polish (external open, a11y)`

## Non-goals (v1)

- Replacing or routing `developer__shell` through the user PTY
- Full VS Code PTY Host process isolation
- Remote/SSH terminals
- Injecting agent commands into the user’s interactive shell without explicit UX
- Bottom-panel layout (rejected for Goose)
- Window-global shared terminal across chats (rejected — must be per session)

## Resolved decisions

- Dock: **right** (not bottom)
- Ownership: **per chat/session**, **multi-tab in v1**
- Background PTYs when switching chats: **keep alive**
- Persist open/closed + width across app restarts: **yes** (per session)
- Process across app quit: **no** — fresh shell(s) on relaunch if panel was open; scrollback not persisted in v1
- Widths: default `420px`, min `280px`, max ~50%
- Cmd+J: closed → open + focus terminal; open → close + focus chat
- Shell: login `$SHELL`, honor `GOOSE_SHELL` when set
- New tab/PTY cwd: session’s current working directory at spawn time (no auto-restart on DirSwitcher)
- Ship: **on by default**, no feature flag — merges or doesn’t
- Background PTY resource cap: **none (A)** — alive until tab/session close or app quit; revisit only if dogfood shows memory pain

## Open Questions

None blocking. Plan is ready for Ready issue + M1 spike.

## Underspecified / holes to watch

- **Persistence store:** electron-store / session metadata / localStorage — follow existing Goose per-session UI prefs if any.
- **Multi-window:** same `sessionId` in two windows — confirm Goose support; if yes, one owner window for that session’s PTYs.
- **Session delete:** kill all tab PTYs for that session + drop persisted panel state.
- **Theme:** xterm colors follow Goose light/dark tokens; palette in implementation.
- **Tab UX details:** new-tab control, close tab, rename? (proposal: + button, close on tab, no rename in v1)

---

## Session Log

### Session 1 (2026-08-03)

- Started: Research whether Goose can get a Cursor-like in-app terminal; surveyed OSS harnesses
- Done: Locked multi-tab v1, Cmd+J open/focus ↔ close/focus-chat, GOOSE_SHELL, spawn cwd = session cwd, on-by-default, widths accepted, **no background PTY cap (A)**
- Next: (superseded by Session 2)

### Session 2 (2026-08-03)

- Started: File issue + implement M1–M3 from worklog on `feat/in-app-terminal`
- Done: Opened [#10921](https://github.com/aaif-goose/goose/issues/10921); implemented `ptyService`, preload IPC, `TerminalPanel`/`TerminalTabView`, BaseChat right dock, Cmd+J shortcut, localStorage persistence, Forge/vite native packaging hooks; typecheck + persistence unit test green
- Next: Dogfood in desktop (Cmd+J); polish external Terminal; move issue to Ready on board; carve PR off `upstream/main`

## Surprises

- Cursor’s **Cmd+J** toggles the *panel*, not strictly “terminal only”; ``Ctrl/` `` is the terminal-specific binding — Goose can map Cmd+J → right terminal panel for muscle memory without copying bottom chrome.
- Most agent harnesses (Continue, Cline, Roo) **don’t** own a PTY — they ride VS Code’s. Goose Desktop is Electron-native, so it must own one (or stay external-terminal-only).
- Goose’s floating composer makes a bottom terminal a worse fit than in Cursor; right dock is the layout-native choice.
- Agent shell streaming already exists in chat — easy to confuse with a user terminal; keep them separate in the issue/PR narrative.
- Per-session PTYs imply a main-process lifecycle map (create/reuse/dispose) tied to `ChatSessionsContainer` / session delete — more than a single xterm instance in the window.
- Closing the panel must **not** unmount xterm/kill PTY — hide with CSS so Cmd+J toggle preserves scrollback and process; kill only on tab close / session delete / quit.
