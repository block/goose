# Build → dogfood → submit checklist

Scope: apply `PATCH.md` to `crates/goose-cli/src/commands/recipe.rs` in a real `aaif-goose/goose`
checkout, build it, swap it into Doug's actual `goose` install, confirm it looks right on his real
recipe directories, then (only after his sign-off) open the upstream PR.

`build.sh` in this folder automates steps 1–5 below as a starting point — it hasn't been run yet
(written from source-reading, not a working build), so treat it as a draft to check over and adapt,
not a script to run blind.

## 1. Get a real checkout

```
gh repo fork aaif-goose/goose --clone=false   # or fork via the GitHub UI if no gh auth here
git clone https://github.com/<your-fork>/goose.git
cd goose
git remote add upstream https://github.com/aaif-goose/goose.git
git checkout -b fix/recipe-list-formatting
```

## 2. Apply the patch

Open `crates/goose-cli/src/commands/recipe.rs` and apply the changes in `../PATCH.md`:
add the `comfy_table` import, replace `handle_list`, add the two new helper functions
(`describe`, `source_path`). Nothing else in the file changes.

## 3. Build + run the existing test suite

```
cargo build -p goose-cli
cargo test -p goose-cli commands::recipe
```

Watch specifically for the `#[cfg(test)] mod tests` block already in `recipe.rs` (uses
`TempDir`-based fixtures) — make sure nothing there broke, and consider adding a case that covers
the new grouping/duplicate-flagging behavior.

## 4. Install the built binary for real dogfooding

```
which goose                          # find the currently-installed binary
cp "$(which goose)" "$(which goose).bak"   # keep a rollback copy
cp target/debug/goose "$(which goose)"     # or target/release/goose for a release build
```

## 5. Dogfood it

```
cd ~/AOF/resources/goose/recipes     # or wherever Doug's actual recipes live
goose recipe list
goose recipe list --verbose
goose recipe list --format json      # confirm this is still the old flat/unchanged shape
```

Compare against `../mockups/after-table.txt` and `../mockups/after-verbose.txt` for the intended
shape — but judge against Doug's *real* recipe set, not the mockup's placeholder names. Confirm with
Doug directly that this is actually better before moving on. If anything looks wrong, restore the
`.bak` binary and iterate on the patch.

## 6. Only after Doug confirms it in daily use — submit upstream

```
git add crates/goose-cli/src/commands/recipe.rs
git commit -s -m "fix(cli): group recipe list output and flag duplicate-name collisions"
git push origin fix/recipe-list-formatting
gh pr create --repo aaif-goose/goose \
  --title "fix(cli): group recipe list output and flag duplicate-name collisions" \
  --body "See description below" --base main
```

Follow `aaif-goose/goose`'s `CONTRIBUTING.md`: DCO sign-off (`-s` above), Conventional Commit title,
keep the diff small and scoped (it already is — one function + two helpers, no new dependency),
and expect an automated Codex review pass on the PR before human maintainers look at it.

PR description should explain the problem (flat, undeduplicated output when a recipe name exists on
more than one search path), show a before/after (the mockups in `../mockups/` are a good starting
point — swap in real example output from the dogfooding step), and note the "no new dependency"
point explicitly since that's usually a fast path to a friendly review.

Also include this line, since the current implementation was built against clig.dev
(https://clig.dev/) — semantic color/glyphs, NO_COLOR/non-TTY degradation via `console`'s built-in
detection, no bordered table — using only crates already in goose-cli's `Cargo.toml`:

> Output follows the clig.dev CLI guidelines (semantic color, NO_COLOR/non-TTY degradation); no new dependencies.

Note: steps 1–2 and 5 above (comfy_table import, `PATCH.md`, `../mockups/after-table.txt`) describe
an earlier bordered-table version of this patch. The implementation actually built and dogfooded in
`recipe.rs` is the later stacked-block rewrite (grouped blocks, wrapped not truncated, `⚠` for
collisions) — this file hasn't been re-synced to that yet. Treat steps 3–4 and 6 as still accurate;
re-verify 1–2 and 5 against the real diff before running through this checklist for real.
