# Handoff: `goose recipe list` formatting patch — ready for build + submit

**From:** Cowork (genbu-dev thread) **Date:** 2026-07-08 **Status:** patch written, not yet compiled or tested. Next step is Claude Code.

## Why this is here, not in genbu-dev

This started as a side conversation in the genbu-dev thread while working the Genbu/Goose fork plan — Doug's *personal, vanilla* Goose CLI install (not Genbu-branded) turned out to have messy `recipe list` output, and it looked like a legitimate small upstream contribution. Moving it here so it doesn't clutter genbu-dev with local-install/dogfooding chatter. This repo (`tortu-goose`) is the staging ground for Doug's Goose config + any upstream contributions in progress.

**Important distinction:** the actual pull request target is `aaif-goose/goose` (upstream), not this repo. `tortu-goose` is just where we stage, build, and validate before that PR goes out.

## The problem

`goose recipe list` has no deduplication logic. If a recipe with the same name exists on more than one search path (very common — Doug hit this with `~/.goose/recipes/` vs `~/AOF/resources/goose/recipes/` after a `GOOSE_RECIPE_PATH` mixup), every hit from every path prints as a separate, indistinguishable flat line. Doug's actual output was 12 lines for 6 unique recipes, with no indication that one file was silently shadowing the other. Doug's reaction: *"That recipe listing is a mess. Like it's hard to read... Might this be a contribution we could make where we just have the standard CLI render cleaner?"*

## What's in `patches/recipe-list-formatting/`

- `PATCH.md` — the actual Rust patch to `crates/goose-cli/src/commands/recipe.rs`'s `handle_list` function (plus two small new helpers). Fetched and read the real upstream source first, so this is written against the actual current code, not a guess. Needs **zero new dependencies** — `comfy-table` (v7) is already in `goose-cli`'s `Cargo.toml`, and the `⚠` warning glyph matches the file's existing `console`-styled `✓`/`✗` conventions.
- `mockups/` — text mockups of before/after CLI output (non-verbose table + verbose mode), so Doug could see the shape before any code was written. Illustrative recipe names — the exact original terminal output wasn't captured verbatim in this thread, so don't take the sample names as literal.
- `BUILD_AND_SUBMIT.md` — step-by-step build → dogfood → PR checklist.
- `build.sh` — starter script automating the build/install/dogfood steps. **Not yet run** — written from source-reading, not from a working build. Treat it as a first draft to adapt, not a verified script.

## Why this didn't get built or tested already

Tried, in the Cowork sandbox this originated in: no Rust toolchain pre-installed, `rustup.rs` is blocked on that sandbox's network allowlist, disk was tight (~3.7GB free) for a workspace with three path-dependent sibling crates plus an Electron UI, and a clone attempt was killed mid-operation by a 45s tool timeout. None of that applies to Doug's own machine — this needs a real Rust toolchain and disk headroom, which is why it's handed off here instead of force-fitting a build into that sandbox.

## What's needed next (Claude Code)

1. Read `patches/recipe-list-formatting/BUILD_AND_SUBMIT.md` and `build.sh`.
2. Fork/branch `aaif-goose/goose`, apply the patch from `PATCH.md`.
3. `cargo build -p goose-cli`, sanity-check the unit tests in `commands/recipe.rs` still pass.
4. Swap the built binary in for Doug's real `goose` install and dogfood `recipe list` / `recipe list --verbose` against his actual recipes directories.
5. Once Doug confirms it looks right in daily use: commit with DCO sign-off, push to a fork, open the PR against `aaif-goose/goose` per their `CONTRIBUTING.md` (Conventional Commit title, small scoped diff, expect AI/Codex review).

Don't open the upstream PR until Doug has actually dogfooded it — that was the explicit ask.
