# Claude Code prompt: build + dogfood + submit `goose recipe list` patch

Copy/paste this into Claude Code, run from (or pointed at) `~/AOF/work/projects/tortu-tools/tortu-goose`.

---

There's a ready-to-build patch staged in this repo at `patches/recipe-list-formatting/`:

- `PATCH.md` — the actual Rust patch to apply to `crates/goose-cli/src/commands/recipe.rs` in a fresh `aaif-goose/goose` checkout (replaces `handle_list`, adds two small helpers `describe`/`source_path`, adds one import — nothing else in the file changes). No new dependency; `comfy-table` is already in `goose-cli`'s `Cargo.toml`.
- `BUILD_AND_SUBMIT.md` — the full fork → apply → build → test → install → dogfood → PR checklist.
- `build.sh` — a draft automation script for the build/install/dogfood steps. It intentionally exits immediately with instructions, because the patch hasn't been applied to a real checkout yet — read it, apply the patch by hand per `PATCH.md`, then comment out the early exit and use it from step 2 onward (or just run the commands manually from `BUILD_AND_SUBMIT.md`).
- `mockups/before.txt`, `mockups/after-table.txt`, `mockups/after-verbose.txt` — illustrative before/after terminal output (placeholder recipe names — compare shape, not literal content, against Doug's real output).

Full background is in `_inbox/HANDOFF_2026-07-08_recipe-list-formatting.md` if useful.

**What to do:**

1. Read `patches/recipe-list-formatting/BUILD_AND_SUBMIT.md`, `PATCH.md`, and `build.sh`.
2. Fork `aaif-goose/goose`, clone it, create branch `fix/recipe-list-formatting`, apply the patch from `PATCH.md`.
3. `cargo build -p goose-cli` and `cargo test -p goose-cli commands::recipe` — confirm the existing test suite still passes; consider adding a test for the new grouping/duplicate-flagging behavior.
4. Back up Doug's current `goose` binary (`cp "$(which goose)" "$(which goose).bak"`), then install the built binary in its place.
5. Dogfood it against Doug's real recipe directories (`goose recipe list`, `goose recipe list --verbose`, `goose recipe list --format json` — confirm JSON output is unchanged).
6. **Stop and confirm with Doug directly that the real output looks right before doing anything else.** Do not open the upstream PR until he's actually used it and signed off — that's an explicit requirement, not a formality.
7. Only after his sign-off: commit with DCO sign-off (`git commit -s`), Conventional Commit title, push to your fork, and open the PR against `aaif-goose/goose` per their `CONTRIBUTING.md` (small scoped diff, expect an automated Codex review pass). PR description should show a before/after using Doug's *real* dogfooded output, not the mockup placeholders.

If anything breaks or looks wrong at any step, roll back the binary from the `.bak` copy and report back rather than pushing forward.
