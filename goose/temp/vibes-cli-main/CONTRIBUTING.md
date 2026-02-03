# Contributing to Vibes CLI

This guide covers how to develop, test, and contribute to the Vibes CLI plugin.

## Development Setup

### Prerequisites

- Node.js 18+ (uses native fetch)
- Claude Code CLI installed
- Git

### Clone and Install

```bash
git clone https://github.com/popmechanic/vibes-cli.git
cd vibes-cli
cd scripts && npm install
```

### Local Development

For testing changes locally without publishing:

```bash
# Reinstall from local directory
./dev-reinstall.sh
```

This script uninstalls the existing plugin and reinstalls from your local working copy.

## Project Structure

```
vibes-cli/
├── .claude-plugin/          # Plugin manifest
│   ├── plugin.json          # Main config (version here)
│   └── marketplace.json     # Marketplace metadata (version here too)
├── skills/                  # Model-invoked skills
│   ├── vibes/SKILL.md       # Core app generation
│   ├── riff/SKILL.md        # Parallel variations
│   ├── sell/SKILL.md        # SaaS transformation
│   └── exe/SKILL.md         # exe.dev deployment
├── commands/                # User-invoked commands
│   ├── sync.md              # Update cache from upstream
│   └── update.md            # Update existing apps
├── scripts/                 # Node.js utilities
│   ├── sync.js              # Cache sync script
│   ├── assemble.js          # JSX → HTML assembly
│   ├── update.js            # App updater
│   └── __tests__/           # Test suite
├── cache/                   # Working cache (gitignored)
└── skills/vibes/cache/      # Default cache (git-tracked)
```

## Running Tests

```bash
cd scripts

# All tests
npm test

# Unit tests only (fastest)
npm run test:unit

# Integration tests (with mocks)
npm run test:integration

# E2E local server
npm run test:e2e:server
```

### Test Structure

- **Unit tests** (`__tests__/unit/`): Pure functions, no I/O
- **Integration tests** (`__tests__/integration/`): Use mocks from `mocks/`
- **E2E tests** (`__tests__/e2e/`): Local server for manual testing

## Adding a New Skill

1. Create `skills/yourskill/SKILL.md` with YAML frontmatter:
   ```yaml
   ---
   name: yourskill
   description: What triggers this skill
   ---
   ```

2. Write the skill instructions in Markdown

3. If the skill needs templates, create `skills/yourskill/templates/`

4. Update CLAUDE.md with the new skill in the File Reference table

## Adding a New Command

1. Create `commands/yourcommand.md` with YAML frontmatter:
   ```yaml
   ---
   name: yourcommand
   description: What this command does
   ---
   ```

2. Write usage instructions in Markdown

3. If the command needs a script, add it to `scripts/`

4. Update CLAUDE.md with the new command

## Version Bumping

When releasing a new version, update **both** files:

1. `.claude-plugin/plugin.json`
2. `.claude-plugin/marketplace.json` (in the `plugins` array)

Both must have matching version numbers.

```bash
# Example: bump to 0.1.18
# Edit plugin.json line 4: "version": "0.1.18"
# Edit marketplace.json plugins array: "version": "0.1.18"
```

## Sync Script Development

The sync script (`scripts/sync.js`) fetches from upstream vibes.diy. Key considerations:

- **Current version**: This plugin uses `0.24.3-dev` with cloud sync features
- **Cache priority**: `skills/vibes/cache/` is the authoritative source
- **Template updates**: Sync updates import maps in SKILL.md files

To test sync changes:

```bash
cd scripts
node sync.js --force
```

## Pull Request Guidelines

1. **One feature per PR**: Keep changes focused

2. **Update documentation**: If you change behavior, update CLAUDE.md

3. **Test your changes**: Run the test suite

4. **Version bump**: Only maintainers bump versions

5. **Commit messages**: Be descriptive, no Claude Code credits

## Code Style

- ES modules (`type: "module"` in package.json)
- Use `async/await` over callbacks
- Error messages should be actionable

## Common Tasks

### Update Import Maps

```bash
cd scripts
node sync.js --force
```

### Test a Generated App

```bash
# Generate an app
# (use /vibes:vibes in Claude Code)

# Open in browser
open index.html

# Check console for errors
```

### Debug Sync Issues

```bash
# Check cache contents
cat cache/import-map.json | jq '.imports'

# Verify versions
grep "0.24.3-dev" skills/vibes/cache/import-map.json
```

## Questions?

- [Discord](https://discord.gg/vnpWycj4Ta) - Join the community
- [GitHub Issues](https://github.com/popmechanic/vibes-cli/issues) - Report bugs
