Update the CLI Commands Guide based on cli-changes.md.

IMPORTANT: You MUST use the text_editor tool to:
1. Read cli-changes.md and gosling-cli-commands.md
2. Update gosling-cli-commands.md with str_replace
3. Write update-summary.md

Remember:
- Document CURRENT state only (not change history)
- Make SURGICAL edits (smallest change needed)
- Only change what's explicitly in cli-changes.md
- Preserve all structure, headings, and content not mentioned in cli-changes.md
- Use EXACT file path from CLI_COMMANDS_PATH environment variable

Do NOT:
- Rewrite existing descriptions or reorganize sections
- Make "improvements" to content not in cli-changes.md

Before finalizing, verify:
1. Did I only change what's in cli-changes.md?
2. Are all section headings (###, ####) unchanged?
3. Did I use str_replace with exact matching?
4. Did I avoid duplicating sections?
