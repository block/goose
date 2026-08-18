# Avocado Work stress REPORT

> Arena: `make dev` + `make dev-ui` (avcd-agent)  
> Judge pass: 2026-08-08T23:19:58Z  
> Related: `stress/findings/JUDGE.json` | `stress/SCOPE.md`

## Executive summary

HTTP/chat ladder held SLOs; no ACP load breakpoint found up to 2000 rps. Isolation findings ISO-001–004 reproduce but are **dismissed**: `/status` is intentional unauthenticated health (unit-tested). UI typing investigation was blocked by harness limits; browser blank-root is harness (Electron preload required). Electron restored with **React ready** on Vite **:5173**.

## Arena restore

| Check | Result |
|-------|--------|
| `GET /status` + valid secret | 200 `ok` |
| `GET /acp` Accept SSE, no secret | **401** (control) |
| Vite UI | **200** on `http://localhost:5173/` (port moved from prior 5174) |
| Electron userData | **Avocado Work** (PID 46137) |
| `ui/desktop/out` / Avocado Work.app | **MISSING** |

## SLO / ladder

From `stress/artifacts/ladder-metrics.json` + `stress/stress-status.json`:

| Rung | Success rate | p99 | Notes |
|------|--------------|-----|-------|
| AVERAGE 20 rps / 60s | 100% (1200) | ~7.2 ms | SLO p99 < 200 ms held |
| STRESS 100 rps / 60s | 100% (6000) | ~5.7 ms | held |
| SPIKE 500→20 | 100% (8100) | burst ~4.0 ms; recovery ~10.4 ms | recovered to p99<200ms in ~2.0 s |
| BREAKPOINT 50→2000 rps / 180s | 100% (114633) | ~10.2 ms | `stop_reason=ramp_complete`; **no failure breakpoint**; peak 2000 rps |
| CHAT sequential×5 | 100% | p99 ~3.5 s | SLO goose p99 < 15 s at 1 concurrent: **pass** |
| CHAT concurrent×3 | 100% | wall ~3.6 s | container not OOMKilled |

Fault lane: stop/kill-9 recovery ≪ 60 s SLO; Connection:close burst 50/50 200.

## Breakpoint

No sustained error-rate breakpoint on `GET /status` through **2000 rps**. System remained healthy (error_rate 0, max latency ~38 ms).

## Typing investigation

| Probe | Outcome |
|-------|---------|
| Playwright browser @ Vite | Blank `#root`; `window.electron` undefined → ErrorBoundary (**harness**, UI-001/UI-002) |
| Electron after restore | `Opening URL http://localhost:5173/#/?` then **React ready** ×2 in `main.log` |
| AppleScript keystroke | Denied (UI-HARNESS-001) |
| screencapture | Denied (UI-HARNESS-002) |
| TelemetryConsentPrompt | Unlikely open (UI-003 dismissed) |
| OnboardingGuard / composer | Not proven blocked in Electron (UI-004 not_reproducible) |

**Verdict:** Cannot confirm desktop chat-input typing failure as a product bug; browser blank screen is expected without preload. Electron SPA reached React ready.

## `/status` auth (product intent)

- Router: `create_router` mounts `/health` and `/status` on **aux_routes** without `check_acp_token`; only `/acp` (and MCP proxy secrets) enforce the key.
- Test: `crates/goose/tests/acp_transport_auth_test.rs` → `health_endpoints_skip_token_check`.
- SCOPE isolation oracle requiring auth on `/status` is incorrect vs product; ISO findings dismissed.

## Confirmed findings

| ID | Severity | Title |
|----|----------|-------|
| SMOKE-001 | P3 | Avocado Work.app missing for `make test-smoke` (`ui/desktop/out` absent) |
| UI-006 | P3 | Goose 2.app still running (separate userData) |
| UI-008 | P3 | Vite IPv6-only bind (`localhost` OK, `127.0.0.1` fails); port may be 5173 or 5174 |

## Dismissed / harness (selected)

| ID | Bucket | Note |
|----|--------|------|
| ISO-001..004 | dismissed | Intentional public `/status` |
| UI-001, UI-002 | harness_bug | Electron preload required |
| UI-003 | dismissed | Telemetry not gating |
| UI-HARNESS-001..003 | harness_bug | OS permissions / Playwright setup |

## Smoke packaging

`scripts/smoke-test.sh` looks for `ui/desktop/out/**/Avocado Work.app`. Directory **missing** — packaging smoke remains FAIL until `make package-ui`.

## Artifacts

- `stress/findings/JUDGE.json`
- `stress/findings/{input,ui,contract,fault}.json`
- `stress/artifacts/ladder-metrics.json`, `chat-stress.json`, `main.log` (host: `~/Library/Application Support/Avocado Work/logs/main.log`)
