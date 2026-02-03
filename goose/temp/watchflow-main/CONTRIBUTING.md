# Contributing to Watchflow

Thanks for considering contributing. Watchflow is a **rule engine** for GitHub—rules in YAML, enforcement on PR and push. We aim for **maintainer-first** docs and code: tech-forward, slightly retro, no marketing fluff. The hot path is condition-based (no LLM for rule evaluation); optional AI is used for repo analysis and feasibility suggestions. See [README](README.md) and [docs](docs/) for the supported logic and architecture.

---

## Direction and scope

- **Rule engine** — Conditions map parameter keys to built-in logic (e.g. `require_linked_issue`, `max_lines`, `require_code_owner_reviewers`). New conditions live in `src/rules/conditions/` and are registered in `src/rules/registry.py` and `src/rules/acknowledgment.py`.
- **Webhooks** — Delivery ID–based dedup so handler and processor both run; welcome comment when no rules file exists.
- **API** — Repo analysis and proceed-with-PR support `installation_id` so install-flow users don’t need a PAT.
- **Docs** — All MD files should speak to engineers: direct, no fluff, immune-system framing (Watchflow as necessary governance, not “another AI tool”).

---

## Getting started

### Prerequisites

- Python 3.12+
- [uv](https://docs.astral.sh/uv/) (recommended) or pip
- OpenAI API key (for repo analysis and feasibility agents)
- LangSmith (optional, for agent debugging)
- GitHub App credentials (for local webhook testing; see [LOCAL_SETUP.md](LOCAL_SETUP.md))

### Development setup

```bash
git clone https://github.com/warestack/watchflow.git
cd watchflow
uv sync
cp .env.example .env
# Add API keys and GitHub App credentials to .env

uv run pytest tests/unit/ tests/integration/ -v   # run tests
uv run python -m src.main                          # start server
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for full env vars and [LOCAL_SETUP.md](LOCAL_SETUP.md) for GitHub App and ngrok.

---

## Code and architecture

- **Conditions** — One class per rule type in `src/rules/conditions/`; each has `name`, `parameter_patterns`, `event_types`, and `evaluate()` / `validate()`. Registry maps parameter keys to condition classes.
- **Rule loading** — `src/rules/loaders/github_loader.py` reads `.watchflow/rules.yaml` from the default branch; normalizes aliases (e.g. `max_changed_lines` → `max_lines`).
- **PR processor** — Loads rules, enriches event with PR files and CODEOWNERS content, passes **Rule objects** (with condition instances) to the engine so conditions aren’t stripped.
- **Task queue** — Task ID includes `delivery_id` when present so handler and processor get distinct IDs per webhook delivery.

---

## Running tests

```bash
uv sync --all-extras
uv run pytest tests/unit/ tests/integration/ -v
```

Run from repo root with the project venv active (or use `just test-local` / see DEVELOPMENT.md) so the correct interpreter and deps are used.

---

## Pull requests

1. Branch from `main` (or the current target branch).
2. Keep changes focused; prefer multiple small PRs over one large one.
3. Ensure tests pass and pre-commit hooks (ruff, etc.) pass.
4. Use conventional commit style where possible (e.g. `fix(rules): preserve conditions in engine`).

---

## Docs

- **Tone** — Tech-forward, slightly retro, maintainer-first. Speak to engineers, not marketing. “Immune system” framing: Watchflow as necessary governance, not another AI tool.
- **Accuracy** — Parameter names and conditions in docs must match the code (see `src/rules/registry.py` and condition modules).
- **Examples** — Use real parameter names: `require_linked_issue`, `max_lines`, `require_code_owner_reviewers`, `no_force_push`, etc.

---

## Questions

- [GitHub Discussions](https://github.com/warestack/watchflow/discussions)
- [GitHub Issues](https://github.com/warestack/watchflow/issues)
