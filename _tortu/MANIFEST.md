# _tortu — Tortu's custom goose

This directory holds everything needed to reproduce Doug's goose setup on a
fresh machine: `git clone` this fork, run `_tortu/bootstrap.sh`, and it's
ready — no manual reconfiguring.

This fork (`tortu-forks/goose`) is the persistent home for that setup.
Upstream is `aaif-goose/goose` (remote `origin`); this fork is remote `fork`.
Patches are dogfooded and committed here first, then opened as narrow PRs
against `origin/main` once validated.

## Layout

- `docs/` — design notes and decisions about the fork/tortu setup itself.
- `_inbox/` — task handoffs and prompts used to build patches (historical
  record of how a given patch came to be).
- `patches/<name>/` — one folder per patch: the patch spec, build/submit
  checklist, and any supporting mockups. Mirrors what actually got applied
  to the goose source under `crates/`.
- `recipes/` — goose recipe YAML files. `GOOSE_RECIPE_PATH` should point
  here.
- `config/` — everything that goes in `~/.config/goose/`:
  - `config.yaml` — goose's main config.
  - `custom_providers/` — custom provider definitions (e.g. `custom_omlx.json`).
    Provider files reference secrets only by env var name (`api_key_env`),
    never by value.
  - `skills/` — custom skill definitions.
  - `goosehints` — copied from the kaminari project's `.goosehints`; this
    fork carries its own copy so it's self-contained and doesn't depend on
    another repo being checked out.
  - `secrets.env.example` — template listing required credential env vars
    (names only, no values). Tracked.
  - `secrets.env` — the real credential values. **Gitignored, never
    committed.** Created by `bootstrap.sh` (interactive prompt) or by hand:
    `cp secrets.env.example secrets.env` and fill it in.

## New machine setup

```sh
git clone https://github.com/tortu-forks/goose.git
cd goose
_tortu/bootstrap.sh
```

`bootstrap.sh` installs the Rust toolchain and cmake if missing, creates
`secrets.env` interactively if it doesn't exist, copies `_tortu/config/*`
into `~/.config/goose/`, builds `goose-cli` in release mode, and installs
the binary to `~/.local/bin/goose`. It prints the `PATH` and
`GOOSE_RECIPE_PATH` exports to add to your shell profile if they're not
already set.

## Adding a new patch

1. Branch off `origin/main` (not this fork's `main`) so the PR diff stays
   clean: `git fetch origin && git checkout -b fix/<name> origin/main`.
2. Apply and dogfood the change; add a `patches/<name>/` folder here with
   the patch write-up.
3. Once validated and signed off, commit with DCO (`git commit -s`), push
   to `fork`, open the PR against `aaif-goose/goose`. Use the standard
   patch-attribution header/description (see Claude memory:
   `patch-attribution-format`).
4. Separately, merge the same change into this fork's own `main` so the
   dogfood build stays current.
