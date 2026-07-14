**Describe the bug**

When using an ACP CLI provider (e.g. Claude Code via `claude-acp` / `claude-agent-acp`), the context-window indicator in the Desktop chat input always shows a **128k** denominator (e.g. `0 / 128k`), regardless of which model is selected — even for models with 200k or 1M context windows. Switching models in the model picker does not change the displayed limit.

The real context window size **is** available: the ACP adapter reports it, and the backend uses it — only the UI display drops it.

---

**To Reproduce**

1. Configure the `claude-acp` provider (Claude Code via `@agentclientprotocol/claude-agent-acp`).
2. Select any model — `current`, `default`, `opus[1m]`, `claude-sonnet-4-6[1m]`, etc.
3. Start a chat session.
4. Observe the token counter in the chat input: the denominator is always `128k`, even for 1M-context models, and never changes when switching models.

---

**Expected behavior**

The context-window indicator should reflect the actual context window of the active model, as reported by the ACP agent's `usage_update` notifications (`size` field).

---

**Root cause analysis**

The data exists end-to-end but is dropped at the last hop:

1. **Adapter reports it**: `claude-agent-acp` (v0.57.0) sends `session/update` notifications with `sessionUpdate: "usage_update"` carrying both `used` and `size: session.contextWindowSize` (the model's real window).

2. **Backend uses it**: `AcpProvider::get_context_limit()` in `crates/goose/src/acp/provider.rs` stores `usage.size` (from `SessionUpdate::UsageUpdate`) in `context_size` and returns it when non-zero (added in a3bdb918e, #9455). So backend context management works against the correct window — the bug is display-only.

3. **UI drops it**: in `ui/desktop/src/acp/adapter/gooseSessionNotifications.ts`, the `usage_update` case maps `update.used` and accumulated token fields into `tokenState`, but **discards `update.size`**. Similarly `ui/desktop/src/acp/sessionNotificationAdapter.ts` returns `[]` for `usage_update`.

4. **Fallback chain can't help**: `ChatInput.tsx` resolves `tokenLimit` via (1) predefined models from env → (2) canonical model registry → (3) provider metadata `known_models` → (4) `TOKEN_LIMIT_DEFAULT = 128000`. ACP CLI provider model IDs (`current`, `default`, `opus[1m]`, `claude-sonnet-4-6[1m]`, …) miss all three lookups: they aren't in the canonical registry under those IDs, and ACP providers ship with an empty static model list (models are fetched live from the agent's session config options). So the UI always lands on the 128000 fallback.

This affects **all ACP CLI providers** (claude-acp, codex-acp, copilot-acp, amp-acp, pi-acp), since none of their live-fetched model IDs resolve through the static lookup chain.

---

**Suggested fix**

Forward `update.size` from the `usage_update` notification into `tokenState` (e.g. as `contextLimit`) in `gooseSessionNotifications.ts`, and have the chat input prefer a session-reported limit over the static lookup chain when present. This would also make the denominator track mid-session model switches (e.g. 200k → 1M), which a static registry lookup cannot do.

---

**Please provide the following information**
- **OS & Arch:** macOS (Apple Silicon)
- **Interface:** UI (Desktop)
- **Version:** v1.41.0
- **Extensions enabled:** developer, memory (not relevant — reproduces with any)
- **Provider & Model:** Claude Code (claude-acp via @agentclientprotocol/claude-agent-acp 0.57.0) — any model

---

**Additional context**

Verified by inspecting the adapter's emitted notifications (`usage_update` includes `size`), the v1.41.0 backend (`get_context_limit` returns captured size), and the v1.41.0 UI handlers (size discarded). No commit on `main` currently wires `update.size` into the UI (`git log -S "update.size" -- ui/desktop` is empty as of this filing).

