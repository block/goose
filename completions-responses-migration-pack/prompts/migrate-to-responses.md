### Migration to OpenAI Responses API (Codex automation brief)

You are migrating this repository from legacy OpenAI Completions/Chat Completions to the unified Responses API using gpt-5.

#### Goals
- Enumerate all call sites using legacy endpoints/SDKs.
- Propose a per-language migration plan and sequencing.
- Apply safe, minimal edits to switch to Responses API.
- Update callers to consume the Responses output schema; no backcompat wrappers.
- Run tests/lints; fix trivial breakages introduced by the migration.
- Prepare small, reviewable change sets and provide a final summary with diffs (do not commit).

#### Guardrails
- Only modify files inside the git workspace. Never write outside.
- Do not preserve backward-compatibility shims; migrate code to the new API shape.
- Do not leave tombstone/transition comments or backup files.
- Preserve streaming semantics if previously used; otherwise use non-streaming.
- Ask for approval before running commands or network calls if approval mode.
- Do not run `git add`/`git commit`/`git push`; leave version control to the bootstrap script. Produce working-tree edits only.

#### Reference pack
- Read `docs/responses-cheatsheet.md`, `docs/migration-notes.md`.
- Use `docs/language-recipes/*.md` as idioms for Node/Python/others.

#### Heuristics (detect and rewrite)
- Node: `createCompletion`, `OpenAIApi`, `ChatCompletion` → `OpenAI` + `client.responses.create(...)`.
- Python: `openai.Completion.create`, `openai.ChatCompletion.create` → `OpenAI()` + `client.responses.create(...)`.
- cURL/infra: `POST /v1/completions` → `POST /v1/responses`; map `prompt`→`input`, `max_tokens`→`max_output_tokens`.
- Tools: convert function-calling to `tools` with JSON Schema; use `tool_choice`; return tool results as a `tool` role turn in the next request.
- Multi-turn: maintain conversation history in the app; pass prior turns via `input` items.
- Formatting: replace Chat’s top-level `response_format` with `text.format` in Responses. Prefer a JSON Schema wrapper with `format.name` (e.g., `text: { format: { type: "json_schema", name: "Output", json_schema: { strict, schema } } }`). Remove the legacy field; avoid plain string formats.
- Content items: replace Chat `content[].type: "text"` with Responses `content[].type: "input_text"` for user/system turns; for assistant/tool outputs, prefer `content[].type: "output_text"`.
- GPT‑5 reasoning effort: if the prior model is o‑series (`/^o/`), set `reasoning: { effort: "medium" }`; if GPT‑series (`/^gpt-/`), set `reasoning: { effort: "minimal" }`. Respect any explicit `reasoning.effort` provided.

#### Acceptance
- All legacy Completions/ChatCompletion calls replaced with Responses equivalents.
- Imports/initialization updated; builds/tests pass or clear TODOs noted.
- Callers updated to consume Responses outputs (no legacy shapes retained).
- If migrating to gpt-5, ensure temperature is omitted or set to 1.
- Replace any top-level `response_format` usage with `text.format` and update parsing accordingly (e.g., `JSON.parse(res.output_text)`).
- Ensure the OpenAI SDK is upgraded to a version supporting Responses (e.g., Node `openai@latest`, Python `pip install -U openai`) and dependencies reinstalled.
- Project builds cleanly (e.g., `npm run build` / `pnpm build` / `yarn build`) when applicable.
- Always set `store: false`; do not rely on previous message IDs or server-stored context; keep conversation state in-app.
- Always include encrypted reasoning tokens; do not emit plaintext chain-of-thought.
- Summary includes edited files, before/after counts of legacy calls, and next steps.


