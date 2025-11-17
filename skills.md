# Skills Extension

The Skills extension enables goose to discover and use reusable skills defined in SKILL.md files.

## Implementation

The skills extension is implemented as a platform extension in `crates/goose/src/agents/skills_extension.rs`.

### Key Components

1. **YAML Frontmatter Parser**: Parses skill metadata (name and description) from YAML frontmatter in SKILL.md files
2. **Skills Discovery**: Scans multiple directories for skills at initialization
3. **Instructions Generation**: Creates dynamic instructions listing all available skills
4. **loadSkill Tool**: Loads skill content and lists supporting files

### Skills Discovery

The extension scans the following directories for skills (in order):

1. `~/.claude/skills/` - Claude-compatible skills directory
2. `~/.config/goose/skills/` - Goose config directory (platform-specific)
3. `{working_dir}/.claude/skills/` - Project-level Claude skills
4. `{working_dir}/.goose/skills/` - Project-level Goose skills

Each skill must be in its own directory containing a `SKILL.md` file.

## Usage

### Skill File Format

A skill is defined by a `SKILL.md` file with YAML frontmatter:

```markdown
---
name: your-skill-name
description: Brief description of what this skill does and when to use it
---

# Your Skill Name

## Instructions
Provide clear, step-by-step guidance.

## Examples
Show concrete examples of using this skill.
```

### Supporting Files

Skills can include supporting files alongside `SKILL.md`:

```
my-skill/
├── SKILL.md (required)
├── reference.md (optional documentation)
├── examples.md (optional examples)
├── scripts/
│   └── helper.py (optional utility)
└── templates/
    └── template.txt (optional template)
```

Reference these files from SKILL.md:

```markdown
For advanced usage, see [reference.md](reference.md).

Run the helper script:
```bash
python scripts/helper.py input.txt
```
```

### How Goose Uses Skills

When goose starts, the skills extension:

1. Scans all skill directories
2. Parses each SKILL.md file to extract name and description
3. Generates instructions listing all available skills
4. Makes the `loadSkill` tool available

Goose will see instructions like:

```
You have these skills at your disposal, when it is clear they can help you solve a problem or you are asked to use them:

- skill-one: Description of skill one
- skill-two: Description of skill two
```

When goose needs a skill, it calls `loadSkill` with the skill name, which returns:
- The full skill body (content after frontmatter)
- List of supporting files in the skill directory
- Full path to the skill directory
- Instructions to use view file tools or dev extension to access supporting files

### Example: Creating a Skill

1. Create a skill directory:
```bash
mkdir -p ~/.goose/skills/code-review
```

2. Create `SKILL.md`:
```bash
cat > ~/.goose/skills/code-review/SKILL.md << 'EOF'
---
name: code-review
description: Perform thorough code reviews following best practices
---

# Code Review Skill

## Instructions

1. Read the code changes
2. Check for:
   - Code style and formatting
   - Potential bugs or edge cases
   - Performance issues
   - Security vulnerabilities
   - Test coverage
3. Provide constructive feedback
4. Suggest improvements

## Checklist

- [ ] Code follows project style guide
- [ ] No obvious bugs or logic errors
- [ ] Edge cases are handled
- [ ] Tests are included
- [ ] Documentation is updated
EOF
```

3. Use the skill:
```bash
goose run -t "please use the code-review skill to review my latest changes"
```

## Extension Configuration

The skills extension is enabled by default as a platform extension. It requires no additional configuration.

To disable it, add to your goose configuration:

```yaml
extensions:
  skills:
    enabled: false
```

## Testing

The extension includes comprehensive unit tests:

- YAML frontmatter parsing (valid, missing, unclosed)
- Skill file parsing with supporting files
- Skills discovery from multiple directories

Run tests:
```bash
cargo test -p goose skills_extension
```
