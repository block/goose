# Testing Guide

This guide explains how to test the CLI command tracking automation locally and in GitHub Actions.

## Prerequisites

- Python 3.7+
- Rust toolchain (for building goose)
- jq (for JSON processing)
- goose CLI installed (for running recipes)
- Git with access to goose repository

## Local Testing

### Step 1: Set Up Environment

```bash
cd /path/to/cli-command-tracking

# Set the goose repository path
export GOOSE_REPO=/path/to/goose

# Create output directory
mkdir -p output
```

### Step 2: Test Extraction Script

Test the extraction with a specific version:

```bash
# Test with a release version
./scripts/extract-cli-structure.sh v1.19.0 > output/test-extraction.json

# Verify output
jq '.version, .commands | length' output/test-extraction.json

# Inspect a specific command
jq '.commands[] | select(.name == "session")' output/test-extraction.json

# Verify skipped commands are excluded
jq '.commands[].name' output/test-extraction.json | grep -v term
```

**Expected output:**
- Valid JSON structure
- Version number extracted correctly
- All commands captured (14+ commands, excluding skipped ones like `term`)
- Subcommands nested properly
- Options parsed with all fields

**Common issues:**
- Rust not installed: Install via rustup
- Build fails: Check Cargo.toml dependencies
- Timeout errors: Increase timeout in script if needed
- Keychain prompt: See "Keychain Access" section below

### Step 3: Test Diff Script

Compare two CLI structures:

```bash
# Extract from two versions
./scripts/extract-cli-structure.sh v1.14.0 > output/old-cli-structure.json
./scripts/extract-cli-structure.sh v1.15.0 > output/new-cli-structure.json

# Run diff
python3 scripts/diff-cli-structures.py \
  output/old-cli-structure.json \
  output/new-cli-structure.json \
  > output/cli-changes.json

# Check results
jq '.has_changes, .summary' output/cli-changes.json

# View specific changes
jq '.changes.commands.added' output/cli-changes.json
jq '.changes.commands.modified[0]' output/cli-changes.json
jq '.breaking_changes' output/cli-changes.json
```

**Expected output:**
- `has_changes: true` if versions differ
- Summary with counts of changes
- Detailed changes in structured format
- Breaking changes categorized

### Step 4: Test AI Synthesis Recipe

Generate human-readable documentation:

```bash
cd output

# Run synthesis recipe
goose run --recipe ../recipes/synthesize-cli-changes.yaml

# Check output
ls -lh cli-changes.md
head -50 cli-changes.md
```

**Expected output:**
- `cli-changes.md` file created
- Markdown formatted properly
- Breaking changes listed first
- Examples provided for complex changes
- When testing AI workflows, ensure any content sent via the `store_comment` tool does not contain triple-backtick code fences (```), even though regular backticks in markdown files like `cli-changes.md` are allowed.

### Step 5: Test Documentation Update Recipe

Update the actual documentation:

```bash
cd output

# Set path to documentation file
export CLI_COMMANDS_PATH=/path/to/goose/documentation/docs/guides/goose-cli-commands.md

# Run update recipe
goose run --recipe ../recipes/update-cli-commands.yaml

# Check outputs
ls -lh update-summary.md
cat update-summary.md

# Verify documentation was updated
git diff $CLI_COMMANDS_PATH
```

### Step 6: Test Full Pipeline

Run the complete end-to-end pipeline:

```bash
cd /path/to/cli-command-tracking

# Set documentation path (optional - only needed for update step)
export CLI_COMMANDS_PATH=/path/to/goose/documentation/docs/guides/goose-cli-commands.md

# Run pipeline
./scripts/run-pipeline.sh v1.14.0 v1.15.0

# Check all outputs
ls -lh output/
```

**Expected output:**
- All intermediate files created
- Pipeline completes without errors
- Summary shows changes detected
- `cli-changes.md` generated

## GitHub Actions Testing

### Test in Fork

1. **Fork the repository** (if not already done)

2. **Copy automation files** to your fork:
   ```bash
   cp -r /path/to/cli-command-tracking \
         /path/to/forked-goose/documentation/automation/
   
   cp /path/to/goose/.github/workflows/docs-update-cli-ref.yml \
      /path/to/forked-goose/.github/workflows/
   ```

3. **Set up secrets** in your fork:
   - Go to Settings → Secrets and variables → Actions
   - Add `ANTHROPIC_API_KEY` secret

4. **Set up variables** (optional):
   - Add `GOOSE_PROVIDER` variable (default: anthropic)
   - Add `GOOSE_MODEL` variable (default: claude-opus-4-5)

5. **Trigger workflow manually**:
   - Go to Actions → "Update CLI Documentation"
   - Click "Run workflow"
   - Set `dry_run: true` for testing
   - Optionally specify versions to compare

### Dry Run Mode

Test without creating PR:

1. Trigger workflow with `dry_run: true`
2. Review outputs in workflow logs
3. Download artifacts to inspect generated files
4. Validate changes are correct

### Workflow Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `old_version` | Previous version tag | Auto-detect from releases |
| `new_version` | New version tag | HEAD |
| `dry_run` | Generate files but don't create PR | true |

### Reviewing Artifacts

After workflow runs:

1. Go to the workflow run page
2. Download the artifacts ZIP
3. Extract and review:
   - `old-cli-structure.json` - Previous CLI structure
   - `new-cli-structure.json` - New CLI structure
   - `cli-changes.json` - Detected changes
   - `cli-changes.md` - Human-readable documentation
   - `pipeline.log` - Execution log

## Testing with Known Changes

To validate the automation works correctly, test with versions that have known CLI changes.

### Finding Test Versions

```bash
cd /path/to/goose

# Check git history for CLI changes
git log --oneline --all -- crates/goose-cli/src/cli.rs | head -20

# Look for commits that added/removed/modified commands
git show <commit-hash>:crates/goose-cli/src/cli.rs | grep "enum Command" -A 30
```

### Test Case: New Command Added

If you know a version added a new command:

```bash
./scripts/run-pipeline.sh v1.13.0 v1.14.0
jq '.changes.commands.added' output/cli-changes.json
```

### Test Case: Option Modified

If you know a version modified options:

```bash
./scripts/run-pipeline.sh v1.14.0 v1.15.0
jq '.changes.commands.modified' output/cli-changes.json
```

### Test Case: No Changes

Test with same version (should show no changes):

```bash
./scripts/run-pipeline.sh v1.14.0 v1.14.0
jq '.has_changes' output/cli-changes.json
# Should output: false
```

## Validation Checklist

Before considering the automation complete:

### Extraction Script
- [ ] Handles all command types (simple, with subcommands, with aliases)
- [ ] Parses all option types (short, long, with values, flags)
- [ ] Captures defaults and possible values
- [ ] Works with commands that have no description
- [ ] Handles nested subcommands (2+ levels)
- [ ] Builds goose from git tags correctly

### Diff Script
- [ ] Detects added commands
- [ ] Detects removed commands
- [ ] Detects modified options
- [ ] Detects changed help text
- [ ] Detects changed defaults
- [ ] Detects changed possible values
- [ ] Categorizes breaking changes correctly

### AI Recipes
- [ ] Generates readable documentation
- [ ] Provides migration guidance
- [ ] Uses correct markdown formatting
- [ ] Avoids backticks (security constraint)
- [ ] Includes relevant examples
- [ ] Uses text_editor tool to write files

### Pipeline
- [ ] Runs end-to-end without errors
- [ ] Handles "no changes" case
- [ ] Creates all expected output files
- [ ] Filters goose session output correctly

### GitHub Actions
- [ ] Workflow triggers correctly
- [ ] Builds goose for both versions
- [ ] Uploads artifacts
- [ ] Creates PR when changes detected
- [ ] Respects dry_run mode
- [ ] Works in forks (fetches upstream tags)

## Troubleshooting

### Keychain Access (macOS)

On macOS, running `goose --help` or `goose --version` may prompt for keychain access. This happens because goose tries to access stored credentials on startup.

**Local workaround:** Allow the keychain access when prompted.

**CI consideration:** GitHub Actions runners don't have a keychain, so this may need to be handled. Check existing goose workflows for patterns - there may be a `keyring: false` config option or environment variable to disable credential loading.

**TODO:** Investigate if this blocks CI execution and document the solution.

### Build fails for old version

Some old versions may have different dependencies:

```bash
# Check if version exists
git tag | grep v1.14.0

# Try building manually
git worktree add /tmp/goose-test v1.14.0
cd /tmp/goose-test
cargo build --release
```

### Extraction timeout

Increase timeout in `extract-cli-structure.py`:

```python
result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)  # Increase from 10
```

### Diff shows unexpected changes

Check if help text formatting changed:

```bash
# Compare raw help output
./old-goose session --help > old-help.txt
./new-goose session --help > new-help.txt
diff old-help.txt new-help.txt
```

### AI recipe fails

Check input files exist and are valid:

```bash
ls -lh output/cli-changes.json output/old-cli-structure.json output/new-cli-structure.json
jq empty output/cli-changes.json  # Validates JSON
```

### Workflow fails in fork

Ensure:
- `ANTHROPIC_API_KEY` secret is set
- Upstream tags are fetched (workflow does this automatically)
- Rust toolchain is available

## Manual Verification

After automation runs, manually verify:

1. **Accuracy**: Do detected changes match actual CLI changes?
2. **Completeness**: Are all changes captured?
3. **Documentation**: Is the updated documentation accurate and clear?
4. **Examples**: Do all examples still work?
5. **Style**: Is formatting consistent with existing docs?

## Test Data

Keep test data for regression testing:

```bash
# Save known-good outputs
mkdir -p test-data
cp output/cli-changes.json test-data/v1.14.0-to-v1.15.0-changes.json
cp output/cli-changes.md test-data/v1.14.0-to-v1.15.0-changes.md
```

Use these to verify future changes don't break existing functionality.
