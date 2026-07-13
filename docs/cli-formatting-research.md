# CLI message formatting — research notes

This document captures how message formatting currently works in the goose
CLI (`crates/goose-cli`) and the Ink-based TUI (`ui/text`), plus a quick look
at how Claude Code and Codex format their terminal output. It is the basis
for the formatting-improvement plan that follows in this branch.

## 1. How the CLI (`crates/goose-cli`) formats messages today

**Renderer:** `crates/goose-cli/src/session/output.rs`
**Event loop:** `crates/goose-cli/src/session/mod.rs` (`process_agent_response`)
**Streaming markdown buffer:** `crates/goose-cli/src/session/streaming_buffer.rs`

### Pipeline

```
main.rs → cli.rs → build_session → CliSession::interactive()
  → input::get_input() (rustyline "> " prompt)
  → handle_message_input() → push_message (no echo) → show_thinking()
  → process_agent_response()
      → agent.reply() stream of AgentEvent
      → output::render_message_streaming() dispatches per MessageContent
          Text        → MarkdownBuffer.push() → print_markdown() → bat::PrettyPrinter
          ToolRequest  → flush buffer → render_tool_request()
          ToolResponse → flush buffer → render_tool_response()
          Thinking     → render_thinking_streaming() (hidden unless GOOSE_CLI_SHOW_THINKING)
      → flush_markdown_buffer_current_theme()
  → hide_thinking(), print dim "⏱ elapsed"
```

### Styling stack

- **`console`** crate for all inline styling (`style(...).dim()/.bold()/.red()/...`).
- **`bat::PrettyPrinter`** renders all assistant markdown (headers, bold, code,
  lists, links) via a named syntect theme. Theme is one of `Theme::{Light,
  Dark, Ansi}` (`GOOSE_CLI_THEME`, default `Ansi` → bat theme `base16`).
- **`comfy-table`** turns markdown pipe-tables into ASCII tables before
  handing them to `bat`.
- **`cliclack`** for the "thinking" spinner and confirmation/elicitation
  prompts.
- **`indicatif`** for MCP notification spinners/progress bars.
- **`rustyline`** for the input prompt (`> `) — no styling on user input.

### Per-element formatting (current state)

| Element | Indent | Color/style | Notes |
|---|---|---|---|
| User message | none | none | Not rendered at all during live chat — only pushed to history. Only shown via `bat` markdown on `/resume` replay. |
| Assistant text | 0 | theme-dependent (bat) | No leading glyph, no visual separation from surrounding tool blocks besides blank lines. |
| Tool call header | 2 spaces | `▸` dim, tool name dim, extension `.magenta().dim()` | Preceded by blank line + 40-char `─` dim rule. |
| Tool params | 4 spaces × depth (`INDENT`) | keys dim, strings green, numbers/bools yellow, null dim | `print_params`, recursive. |
| Tool output/result | 4 spaces | `.dim()` per line | Capped at 20 lines unless `/toggle` or `GOOSE_SHOW_FULL_OUTPUT`. |
| Tool error | 4 spaces | `.red().dim()` | |
| Thinking header | 0 | `"Thinking:"` dim italic | Content is raw dim text, not markdown. Hidden by default. |
| Error | 2 spaces | `"error:"` red bold + plain message | `render_error`. |
| Session header | 2 spaces | ASCII goose art, `●` green, cyan model name, dim metadata | `display_session_info`. |
| Context bar | 2 spaces | green/yellow/red bar by usage % | `display_context_usage`. |
| Elapsed time | 2 spaces | dim, `⏱` prefix | Printed after each turn. |

### Gaps identified

1. **No visual distinction between user and assistant text** — user input
   isn't echoed/styled at all during live chat.
2. **Tool call chrome is the same 3 colors regardless of status** — there is
   no visual signal for pending/running vs. success vs. failure the way the
   TUI's colored status dots (`○ ◑ ● ✗`) provide.
3. **Palette is inconsistent** — tool chrome uses fixed `console` colors
   (dim/magenta/green/yellow/red) independent of the light/dark/ansi theme,
   while assistant markdown color comes entirely from whichever `bat` theme
   is active. The two rarely match.
4. **No consistent left margin** — assistant text sits at column 0, tool
   headers at column 2, params/output at column 4; nothing lines up under a
   common gutter the way Claude Code/Codex align role markers.
5. **No `NO_COLOR`/non-color styling test coverage**, and no unit tests at
   all for the visual shape of tool call or error output (only for
   `shorten_path`, `format_elapsed_time`, streaming-buffer edge cases).

## 2. How the Ink TUI (`ui/text`) formats messages

**Palette:** `ui/text/src/colors.tsx`

```ts
CRANBERRY      = "#C0354A"  // accent / errors / user prompt
TEAL           = "#3A7D7B"  // success / ready status
GOLD           = "#C4883A"  // in-progress / selection
TEXT_PRIMARY   = "#E8E4DF"  // body text
TEXT_SECONDARY = "#8FA4BD"  // titles / labels
TEXT_DIM       = "#5A6D84"  // secondary/hint text
RULE_COLOR     = "#2E3D54"  // borders/rules
```

### Per-element formatting

| Element | Indent/prefix | Color | Spacing |
|---|---|---|---|
| User message | `❯ ` prefix | Prefix cranberry bold; text `TEXT_PRIMARY` bold | 1 blank line before |
| Assistant markdown | none (marked-terminal `tab: 2` for nested lists) | terminal default + marked-terminal ANSI | 1 blank line before each chunk |
| Tool call (collapsed) | rounded box, full width | border `CEDAR`/dim (normal), cranberry (failed), gold (selected); title `TEXT_SECONDARY` bold | 1 blank line before, fixed 3-line box |
| Tool status dots | — | `○` dim (pending), `◑` gold (in-progress), `●` teal (completed), `✗` cranberry (failed) | — |
| Tool call (expanded) | rounded box, paddingX 1 | border gold; labels `TEXT_SECONDARY` bold; body `TEXT_PRIMARY` | 1 blank line between sections |
| Error | flush left, `⚠ Error: ` header | all cranberry | 1 blank line before |
| Loading/"thinking" | spinner + text | spinner cranberry; status text dim italic | 1 blank line before |
| Input box | rounded border, paddingX 1, marginTop 1 | border `RULE_COLOR`; prompt cranberry bold; text `TEXT_PRIMARY` | — |
| Header/rule | 2-line header + rule | title bold; status teal/cranberry/dim; separators `RULE_COLOR` | — |

### Key takeaways to bring to the CLI

- A **consistent, small, mostly-grey/blue-grey palette** (`TEXT_PRIMARY`,
  `TEXT_SECONDARY`, `TEXT_DIM`, one accent, one success, one error) rather
  than ad hoc use of `red/green/yellow/cyan/magenta`.
  Terminal 256-color approximations for the CLI:
  - primary text → default/white
  - secondary/title → `cyan`/`blue` (dim)
  - dim/muted → `.dim()`
  - accent (user prompt, spinner) → single accent color
  - success → green, failure → red (kept minimal, only for outcomes)
- **Status dots** (`○ ◑ ● ✗`) as a single, consistent way to show
  pending/running/success/failure for tool calls, instead of always using the
  same `▸` glyph regardless of outcome.
- **One blank line before each logical message block**, never more, never a
  trailing blank line after.
- A **consistent left gutter**: user/assistant/tool-status prefixes should
  line up under the same column so the eye can scan down a single "rail".

## 3. Claude Code and Codex conventions (from public reports/docs)

Neither tool's source is fully available, but public write-ups, GitHub issues
and community deep-dives converge on a consistent set of conventions:

- **Claude Code** ("Claude Code Internals" — Kotrotsos; `how-claude-code-works`;
  TUICommander docs):
  - Renders with Ink (React for terminals); mostly a **grayscale/dim palette**
    for structural chrome, with color reserved for meaning: green =
    success/user, blue = assistant, yellow = tool/pending, red = error, cyan =
    informational, gray = muted text and borders.
  - Each tool call is preceded by a small **colored status dot** (`●`) that
    encodes state via color (dim while running, then green/red on
    completion) — deliberately rendered as its own styled span, separated by
    a plain space from the tool name, to avoid ANSI dim/bold reset bugs.
  - Status/mode chrome lines at the bottom of the screen are consistently
    **indented by 2 spaces** (`\033[2C`).
  - Permission/confirmation prompts use a stronger accent (blue) separator
    and highlighted selection, contrasting with the default gray rule.
- **Codex CLI** (GitHub issues `openai/codex#17879`, `#21130`, `#12200`):
  - Ships with a fairly **flat, low-contrast default** (mostly terminal
    default foreground + gray/dim) and has had repeated user requests for
    *more* contrast between roles — validating that a subtle, mostly-gray
    palette with a couple of clear accent colors (one for user, one for
    assistant/system) is the right target rather than many colors.
  - Community-proposed fixes for role clarity: **bold accent-colored prefix**
    for the user role, **consistent indentation width** across
    user/assistant text so the transcript stays visually aligned, and a
    **subtle background tint** for user input where terminal background
    detection allows it.
  - Tool call / exec output is shown **inline**, generally indented and
    dimmed relative to the primary response text, with exit code/duration
    metadata appended to the header line rather than mixed into the body.

### Common threads to adopt

1. A restrained, mostly gray/dim palette; color used sparingly and
   *semantically* (one accent for the user, one for status/success, one for
   error) rather than a different named color per tool kind.
2. **Consistent indentation** that aligns role markers/prefixes into a single
   scannable column.
3. **Status indicated by a small dot/marker**, colored by outcome, rather
   than a static glyph.
4. Tool output rendered **visibly indented and dimmer** than primary
   assistant text, so it reads as "supporting detail" rather than the main
   thread of conversation.
5. **One blank line of separation** between logical turns/blocks, not more.

## 4. Direction for goose CLI formatting changes

Given the above, and that this change is **CLI-formatting-only** (no new
TUI, no rebuild of the rendering pipeline, no changes to `ui/desktop`), the
plan is to:

- Echo the **user's message** in the CLI with a consistent accent-colored
  prompt prefix (mirroring the TUI's `❯`), so the transcript shows clear
  turn boundaries.
- Introduce a small set of **named style helpers** in `output.rs` backed by a
  restrained palette (primary/secondary/dim/accent/success/error) so tool
  chrome and headers stop hard-coding `magenta`/`yellow`/`cyan` ad hoc.
  Keep using `bat` for markdown body text (no change to the markdown engine).
- Add **status-aware markers** for tool calls (running vs. done vs. failed)
  using a colored dot, replacing the fixed `▸` used regardless of outcome.
- Normalize **indentation** so tool header / params / output share one
  consistent left gutter, and align it with the new user-message prefix
  width.
- Keep spacing to **one blank line** between blocks, removing any doubled
  blank lines/rules that don't add information (e.g. the 40-char `─` rule
  before every tool call, which is heavier than the TUI/Claude Code/Codex
  equivalents).

See the accompanying implementation plan and test suite for specifics.
