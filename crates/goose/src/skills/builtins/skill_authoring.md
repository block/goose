---
name: skill-authoring
description: Create, edit, validate, and relocate Agent Skills for Goose/Avocado Work. Use whenever the user asks to create or add a skill, write SKILL.md, make something appear in /skills or the Skills page, fix a skill that is not discovered, or mentions skill authoring paths.
---

# Skill Authoring (Goose / Avocado Work)

## Where to write (CRITICAL)

Project skill (default for repo workflows):

```
{working_dir}/.agents/skills/{name}/SKILL.md
```

Global skill (personal, all projects):

```
~/.agents/skills/{name}/SKILL.md
```

Legacy paths still discovered but do **not** prefer them: `.goose/skills/`, `.claude/skills/`.

### NEVER write skills here

- `skills/{name}/SKILL.md` at project root
- `workspace/skills/{name}/SKILL.md`
- Any path without `.agents`, `.goose`, or `.claude` as the parent of `skills/`

Wrong paths are invisible to `/` autocomplete and the Skills page.

## Name rules

- Lowercase letters, digits, hyphens only
- Max 64 characters
- No leading or trailing hyphen
- Directory name must match frontmatter `name`

## SKILL.md template

```markdown
---
name: my-skill-name
description: What it does AND when to use it (third person, specific triggers)
---

# My Skill Name

## Instructions
...
```

`description` is the discovery trigger — include what the skill does and when to use it.

## Creation workflow

1. Choose scope: project (`.agents/skills/`) vs global (`~/.agents/skills/`).
2. Pick a kebab-case name; folder name = frontmatter `name`.
3. Prefer the `create_skill` tool when available (deterministic canonical path).
4. If writing files directly, write only to the canonical path above.
5. Verify the file exists at `.agents/skills/{name}/SKILL.md`.
6. Tell the user to open Skills or type `/` / `/skills` to confirm discovery.

## Supporting files

Place scripts, templates, or references in the same skill directory and link them from `SKILL.md`. Relative paths resolve from the skill directory.

## Relocating misplaced skills

If a skill was written under `skills/` or `workspace/skills/`, move it:

```
{wrong}/skills/{name}/  →  {working_dir}/.agents/skills/{name}/
```

Preserve `SKILL.md` and any supporting files. Do not leave a duplicate under the wrong path.
