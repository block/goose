# Stress test scope: Avocado Work local stack

Arena: `make dev` + `make dev-ui` in avcd-agent; ACP http://127.0.0.1:3000; Electron Vite ~5174
Reset: `make dev-down` then `make dev`; kill Electron via kill-switch
Seed data: OpenRouter key in .env.local; default model deepseek/deepseek-v4-flash
Surface under test:
- GET /status, /health, /acp (SSE contract)
- goose run CLI chat turns via OpenRouter
- Desktop composer (ChatInput) typing + submit path
- OnboardingGuard / TelemetryConsentPrompt gating
Out of scope: production deploy, packaging installers, shared cloud envs, modifying product code

Oracles:
- Correctness: chat turn returns non-empty model text; status always 200 with valid secret
- Isolation: wrong/missing secret must not get 200 on /status
- Contract: only expected HTTP codes; no hang >30s on status; goose run exits 0 with reply
- Resource: no crash of docker server under concurrent status/chat
- UI: chat-input accepts keystrokes when composer mounted and no modal open; typing failure when composer visible is a finding
- SLO hypothesis: status p99 < 200ms at 20 rps for 60s; goose run single-turn p99 < 15s at 1 concurrent; error rate < 1% on status

Average load: 20 rps /status for 60s; 1 concurrent chat turn
Abort: error rate > 50% for 30s on status, or host memory > 90%
Kill switch: see stress/artifacts/kill-switch.txt
