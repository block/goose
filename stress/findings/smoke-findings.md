# Smoke ladder findings (rung: smoke)

**Verdict: FAIL** (`pass: false`) — arena usable for API/chat smoke; packaging smoke and IPv4 Vite curl need attention.

## Arena setup
- Created `stress/{findings,artifacts}`; kill-switch at `stress/artifacts/kill-switch.txt`
- Killed older **AVCD Agent** tree (forge ~19763 / Electron ~19786, userData `AVCD Agent`); left **Avocado Work** (Electron 37350) running
- ACP: `GET /status` with secret → `ok` (HTTP 200)

## Results
| Step | Result | Detail |
|------|--------|--------|
| A `make test-smoke` | FAIL (exit 2) | 1–3 PASS; **4/10: Avocado Work desktop bundle is missing** |
| B `make validate-openrouter` | PASS | 13 OpenRouter models; compose + goose info OK |
| C `goose run` pong | PASS | Reply `pong`; model `deepseek/deepseek-v4-flash` |
| D 5× concurrent `/status` | PASS | All HTTP 200 `ok` |
| E Vite UI | PASS* | `http://localhost:5174/` → 200; `127.0.0.1:5174` fails (listen on `[::1]` only) |
| F `.env` backends | INFO | `GOOSE_EXTERNAL_BACKEND` set; `GOOSE_EXTERNAL_BACKEND_URL` set |
| G Telemetry | INFO | `GOOSE_TELEMETRY_ENABLED: true` in `~/.config/goose/config.yaml` — TelemetryConsent may gate UI |

## Findings (non-code)
1. **Packaging smoke gap**: `test-smoke` expects a built desktop bundle that is not present in this workspace path.
2. **Vite IPv6-only**: forge/vite listens on `[::1]:5174`; curls to `127.0.0.1` get connection failure — use `localhost` for arena checks.
3. **Telemetry consent risk**: telemetry enabled in goose config; watch for `TelemetryConsentPrompt` blocking ChatInput during UI stress.
4. **Dual Electron conflict resolved**: old `AVCD Agent` userData instance removed; Avocado Work remains sole UI under test.

## Artifacts
- `stress/artifacts/smoke.log`
- `stress/stress-status.json`
- `stress/SCOPE.md`
- `stress/artifacts/kill-switch.txt`
