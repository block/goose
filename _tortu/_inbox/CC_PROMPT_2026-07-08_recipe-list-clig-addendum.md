# Claude Code prompt addendum: apply clig.dev to the recipe-list patch

Paste this into the Claude Code session building the `recipe-list-formatting` patch
(alongside `CC_PROMPT_2026-07-08_recipe-list-formatting.md`).

---

As part of this patch, we've adopted the **clig.dev** standard (Command Line Interface
Guidelines, https://clig.dev/) to guide CLI output — **without adding any dependency**.
Use only what's already in `goose-cli`'s `Cargo.toml` (`console`, `comfy-table`).

Apply the standard to the `recipe list` output you're patching:

1. **Semantic color/glyphs only** — reuse the file's existing `console`-styled
   `✓`/`✗` vocabulary (add `⚠` for the duplicate/shadowed-recipe flag). One accent,
   no decoration.
2. **Degrade gracefully** — when stdout is not a TTY (piped / CI) or `NO_COLOR` is set,
   emit plain, uncolored, un-animated output. Confirm `--format json` stays byte-for-byte
   unchanged.
3. **Restraint over borders** — reconsider the bordered `comfy-table` grid against a
   lighter **aligned-columns** layout (grouped by name, duplicates dimmed/flagged), which
   reads cleaner and less "try-hard." Show Doug **both shapes** during dogfood and let him
   pick before the PR.

Then **note the adoption in the patch submission**: add a line to the PR description, e.g.
*"Output follows the clig.dev CLI guidelines (semantic color, NO_COLOR/non-TTY
degradation); no new dependencies."* Keep the diff small and scoped.
