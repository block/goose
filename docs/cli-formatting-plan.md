# CLI message formatting — implementation plan

Based on `docs/cli-formatting-research.md` and the spec tests in
`crates/goose-cli/src/session/formatting.rs`. Scope is strictly
`crates/goose-cli` visual/formatting code — no TUI work, no changes outside
formatting, no changes to `ui/desktop`.

## Goals

1. Give the user's own messages a visible, consistently styled place in the
   transcript (currently invisible during live chat).
2. Replace the single static `▸` tool marker + 40-char rule with a small,
   status-aware marker (pending/success/error) and a lighter-weight header,
   closer to the TUI's `○ ◑ ● ✗` convention and to Claude Code/Codex's
   colored status dots.
3. Centralize color choices behind a small set of semantic roles
   (primary/secondary/muted/accent/success/error) instead of ad hoc
   `magenta`/`yellow`/`cyan` calls, so the palette reads as one coherent,
   mostly-grey/dim scheme with a couple of clear accents — matching the
   TUI's palette and the "grey chrome + sparing color" pattern used by
   Claude Code/Codex.
4. Normalize indentation so top-level chrome (user prompt, tool header,
   status footer, error) shares one 2-space gutter, and nested content
   (params, tool output) shares the existing 4-space indent — so everything
   lines up under a single rail that's easy to scan.
5. Keep exactly one blank line between message blocks (no doubled
   separators).

## Non-goals

- No new TUI, no Ink/React work.
- No change to `bat`'s light/dark/ansi theme system or how markdown is
  highlighted — only how the already-highlighted output is captured and
  laid out (see "Update" below for the one change that touches this).
- No change to `ui/desktop`.
- No change to JSON / stream-json output modes (`emit_stream_event`) — those
  are structured output for tooling, not human-facing formatting.

**Update (follow-up to goal 4):** the original pass above only indented the
*chrome* (headers/footers/errors/params), leaving ordinary model replies
flush-left and visually disconnected from the rest of the gutter-aligned
transcript. A follow-up change extends goal 4 to indent normal assistant
markdown too, by capturing `bat`'s rendered output via
`PrettyPrinter::print_with_writer` (instead of letting it write directly to
stdout) and post-processing it with `formatting::indent_block`. This still
doesn't touch `bat`'s highlighting/theme behavior, but it does mean the CLI
now owns the final `print!()` of that output instead of `bat`. Because
streamed replies are rendered in several chunks that don't necessarily
align with line boundaries, `indent_block` takes an explicit
`at_line_start` flag so a chunk that only continues the previous chunk's
still-open line isn't re-indented mid-sentence; this state is threaded
through `render_message_streaming` the same way `thinking_header_shown`
already is.

## Steps

1. **`crates/goose-cli/src/session/formatting.rs`** (new, already added with
   failing tests): flesh out the pure formatting functions to their target
   behavior:
   - `GUTTER` (2 spaces), `USER_PROMPT_GLYPH` (`❯`).
   - `ToolStatus { Running, Success, Error }` → `status_glyph` (`○ ● ✗`).
   - `Role { Primary, Secondary, Muted, Accent, Success, Error }` →
     `role_style` returning `RoleStyle { color, dim, bold, italic }`:
     - `Primary`: no color/no decoration (inherits terminal default —
       matches TUI's `TEXT_PRIMARY` and Claude Code's default body text).
     - `Secondary`: bold, no color (tool names, section labels) — kept
       colorless so `Accent` remains the palette's only hue.
     - `Muted`: dim only, no color (param values, hints, secondary detail —
       matches TUI's `TEXT_DIM`).
     - `Accent`: cyan + bold (user prompt glyph, pending tool status).
     - `Success`: green.
     - `Error`: red.
   - `format_user_message_plain`, `format_tool_header_plain`,
     `format_tool_status_line_plain`, `format_error_line_plain`: pure string
     builders matching the glyph/gutter rules above.
   - Add a small `apply(role: Role, text: &str) -> console::StyledObject<&str>`
     helper that turns a `RoleStyle` into an actual `console::style(...)`
     call, used by `output.rs` (not itself deeply unit tested beyond
     `role_style`, since it's a thin, side-effect-free wrapper).

2. **`crates/goose-cli/src/session/output.rs`** — wire the new helpers in:
   - `print_tool_header`: drop the 40-char `─` rule; use
     `format_tool_header_plain` + `apply(Role::Accent, glyph)` for the
     status marker (always `Running`/pending at request time, since the
     CLI prints tool calls before their result is known) and
     `apply(Role::Secondary, ...)` for the tool/extension name.
   - `render_tool_response`: after printing tool output (or immediately, if
     there is no output), print the new status footer via
     `format_tool_status_line_plain` — `Success` (green `●`) on `Ok`,
     `Error` (red `✗`) on `Err`, using `apply(status_role(status), ...)`.
   - `render_error`: rebuild on `format_error_line_plain` +
     `apply(Role::Error, ...)`, keeping the leading/trailing blank line for
     separation.
   - `print_params` / `print_tool_output`: keep the existing 4-space nested
     indent, but recolor via `Role::Muted`/`Role::Secondary` instead of
     hard-coded `.dim()/.green()/.yellow()` so values read consistently with
     the rest of the palette.
   - `render_thinking`/`render_thinking_streaming`: recolor the `Thinking:`
     header via `Role::Muted` (unchanged behavior otherwise — still hidden
     by default).

3. **`crates/goose-cli/src/session/mod.rs`** — echo the user's message:
   - In `handle_message_input` (where the user's text is currently only
     pushed to history), call `output::render_user_message(&content)`
     before showing the thinking spinner, so every submitted turn is
     visible in the transcript with the same accent-colored `❯` prefix used
     by the TUI.
   - Leave `render_message_history` (used for `/resume` replay) rendering
     user text through the same new helper for consistency instead of
     falling back to raw markdown for user turns.

4. **Tests**: extend `crates/goose-cli/src/session/formatting.rs`'s test
   module only as needed to lock in the final `role_style`/`status_glyph`
   mapping (the existing 14 tests already specify the target values). Add
   a couple of `output.rs`-level tests if practical (e.g. a test that
   `render_error`'s underlying string matches `format_error_line_plain`).

5. **Docs**: no user-facing docs changes needed (internal chrome only); the
   two new files in `docs/` capture the research and plan for future
   maintainers.

## Verification

- `cargo fmt`
- `cargo test -p goose-cli` (all tests, including the 14 new ones, green)
- `cargo clippy --all-targets -- -D warnings`
- Manual smoke check: run `goose run -t "list files in /tmp"` (or similar)
  in a real terminal and confirm the transcript reads top-to-bottom with a
  visible user line, status-aware tool markers, and a single blank line
  between blocks.
