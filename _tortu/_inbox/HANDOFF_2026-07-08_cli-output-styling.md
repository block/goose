# Handoff: CLI output styling — research + design direction for Goose CLI

**From:** Cowork (Dev Methods Inquiries thread) **Date:** 2026-07-08 **Status:** research/design direction, not a patch. Feeds future Goose CLI output work + the staged `recipe-list-formatting` patch.

## Why this is here

Doug is doing more CLI work and wants Goose's (and the fleet's) command-line output to be **more readable via color coding and layout — deliberately NOT heavy tables** ("we tried that once"), and to look *clean without looking try-hard*. He asked for (1) galleries of exemplary CLI/TUI output to develop an aesthetic from, and (2) well-respected libraries to invoke. This doc captures that research and turns it into a design direction for Goose CLI output. It is design guidance, not a ready-to-build patch — the concrete first application is the existing `recipe-list-formatting` patch in this repo.

## The core payload: the discipline (this matters more than the library)

The respected reference is **[clig.dev](https://clig.dev/)** (Command Line Interface Guidelines). It speaks directly to "try-hard" — *"it can be easy to overdo it and make your program look cluttered or feel like a toy."* The rules that keep output clean:

- **Color/symbols are semantic, not decorative.** One accent color + status glyphs (`✓`/`✗`/`⚠`/`→`) to guide the eye — not a rainbow. (Goose-cli already uses `console`-styled `✓`/`✗`; extend that vocabulary, don't invent a new one.)
- **Degrade gracefully.** Detect when stdout is not a TTY (piped / CI) and drop color + animations/spinners — "so progress bars don't turn into Christmas trees in CI logs." Respect the `NO_COLOR` env var.
- **Whitespace, alignment, and hierarchy over borders/boxes/tables.** Dim secondary text, one bright key value, a subtle rule between sections, aligned columns *without* box-drawing. This is precisely why the table experiment felt heavy.

## Galleries / exemplars (to build the aesthetic)

- **Terminal Trove** (terminaltrove.com) — curated, categorized gallery of terminal tools/TUIs.
- **awesome-terminal-aesthetics** (kud) — "tools that make the terminal genuinely beautiful."
- **awesome-tuis** (rothgar) — broad curated list.
- Best move: **study tools that nail clean output** — `bat`, `eza`, `delta`, `gh`, and Charm's `glow`/`gum`. Copy their restraint.

## Libraries — Rust first (Goose is Rust)

- **`console`** — already in goose-cli; the existing `✓`/`✗` styling. Baseline for color + glyphs.
- **`comfy-table`** — already in goose-cli's Cargo.toml (used by the recipe-list patch). Fine for genuine tabular data, but see the table caveat below.
- **`owo-colors`** — ergonomic semantic coloring; good for the "one accent color" approach.
- **`indicatif`** — progress bars/spinners *with* built-in TTY-awareness (use its no-TTY handling to satisfy the degrade rule).
- **`crossterm`** — terminal capability detection (TTY, color support) for the graceful-degradation logic.
- **`ratatui`** — only if a command ever becomes a full interactive TUI; overkill for plain output.

**Cross-language aesthetic references** (not for Goose code, but the taste to emulate): **Charm / Lip Gloss** (Go) is the "killer clean without try-hard" north star; **Rich** (Python) is the gold standard on the Python side of Doug's fleet.

Ready-made resource worth cribbing from: **gfargo/tui-design-skill** — a design skill for clean minimal TUIs/CLIs spanning Rust/Go/Python/TS.

## Reconcile with the staged `recipe-list-formatting` patch

That patch renders `goose recipe list` via **`comfy-table`** (a bordered table). Doug's "not tables — we tried that once" almost certainly refers to exactly this. **Flag for whoever builds it:** re-weigh the table against these principles before the PR — a lighter **aligned-columns** layout (grouped by name, duplicates dimmed/flagged, no box borders) plus TTY-degradation may read cleaner and less try-hard than a full `comfy-table` grid. Worth showing Doug both shapes during dogfood before choosing. Don't block that patch on this — just apply the lens.

## Toward a house style

The durable output: a small shared **output-style helper** in goose-cli (semantic color map, status glyphs matching the existing `✓`/`✗`/`⚠`, a section-rule helper, and one `is_tty()`-gated switch that strips color/animation when piped) so every command renders consistently. Same fleet-standard instinct Doug applies elsewhere. clig.dev is the house doc to point contributions at.

## Sources

clig.dev; Terminal Trove; kud/awesome-terminal-aesthetics; rothgar/awesome-tuis; charmbracelet/lipgloss; Textualize/rich; gfargo/tui-design-skill. (Full URLs in the originating Cowork session.)
