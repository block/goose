# Testing Recipe Schema Tracking Automation

This guide covers how to test the recipe schema tracking automation both locally and via GitHub Actions.

## Local Testing

### Prerequisites

- goose CLI installed
- jq installed (for JSON processing)
- Git repository with goose source code

### Manual Pipeline Execution

Test the complete pipeline locally:

```bash
cd documentation/automation/recipe-schema-tracking

# Test with no changes expected
./scripts/run-pipeline.sh v1.14.0 v1.15.0

# Test with changes expected
./scripts/run-pipeline.sh v1.9.0 v1.15.0
```

### Individual Script Testing

Test each script independently:

```bash
# Extract schema from a version
./scripts/extract-schema.sh v1.15.0 > output/test-schema.json

# Extract validation structure
./scripts/extract-validation-structure.sh v1.15.0 > output/test-validation.json

# Compare two validation structures
./scripts/diff-validation-structures.sh output/old.json output/new.json > output/test-changes.json
```

### Recipe Testing

Test the AI recipes:

```bash
# Generate change documentation
cd output
goose run --recipe ../recipes/synthesize-validation-changes.yaml

# Update recipe-reference.md
export RECIPE_REF_PATH=/path/to/recipe-reference.md
goose run --recipe ../recipes/update-recipe-reference.yaml
```

## GitHub Actions Testing

### Test in Your Fork

The workflow can be tested in your fork without affecting the upstream repository.

#### Step 1: Push Branch to Fork

```bash
git push origin your-branch-name
```

#### Step 2: Enable GitHub Actions

1. Go to your fork on GitHub
2. Click "Actions" tab
3. Enable workflows if prompted

#### Step 3: Run Workflow Manually

1. Click "Update Recipe Documentation" workflow (docs-update-recipe-ref.yml)
2. Click "Run workflow" button
3. Select your branch
4. Configure inputs (see test scenarios below)
5. Click "Run workflow"

### Test Scenarios

#### Scenario 1: Dry-Run with No Changes

**Purpose**: Verify the workflow runs successfully when no changes are detected.

**Inputs**:
- `old_version`: `v1.14.0`
- `new_version`: `v1.15.0`
- `dry_run`: `true`

**Expected Results**:
- ✅ Workflow completes successfully
- ✅ "No changes detected" message in summary
- ✅ Artifacts uploaded with extraction results
- ✅ No PR created

#### Scenario 2: Dry-Run with Changes

**Purpose**: Test change detection and documentation generation without creating a PR.

**Inputs**:
- `old_version`: `v1.9.0`
- `new_version`: `v1.15.0`
- `dry_run`: `true`

**Expected Results**:
- ✅ Workflow detects changes (4 validation rules, 1 field removal)
- ✅ Generates `validation-changes.md` with documentation
- ✅ Updates `recipe-reference.md`
- ✅ Artifacts uploaded with all generated files
- ✅ No PR created (dry-run mode)

**Review Artifacts**:
1. Download artifact zip from workflow run
2. Check `validation-changes.md` - should document all changes
3. Check `update-summary.md` - should show what was updated
4. Compare updated `recipe-reference.md` with original

#### Scenario 3: Full Run with PR Creation

**Purpose**: Test end-to-end including PR creation.

**Inputs**:
- `old_version`: `v1.9.0`
- `new_version`: `v1.15.0`
- `dry_run`: `false`

**Expected Results**:
- ✅ Workflow runs successfully
- ✅ Creates PR: `docs/recipe-reference-v1.15.0`
- ✅ PR contains updated `recipe-reference.md`
- ✅ PR description includes change summary and checklist

**Review PR**:
1. Check only `recipe-reference.md` was modified
2. Verify changes match dry-run artifacts
3. Confirm no unintended modifications
4. Test documentation renders correctly

#### Scenario 4: Auto-Detection

**Purpose**: Test automatic version detection (simulates production mode).

**Inputs**:
- `old_version`: *(leave empty)*
- `new_version`: *(leave empty)*
- `dry_run`: `true`

**Expected Results**:
- ✅ Auto-detects two most recent releases
- ✅ Compares them automatically
- ✅ Uploads artifacts

### Reviewing Workflow Results

#### Check Workflow Summary

Each workflow run provides a summary with:
- Version comparison performed
- Whether changes were detected
- Dry-run mode status
- Links to artifacts

#### Download and Review Artifacts

Artifacts include:
- `old-validation-structure.json` - Extracted from old version
- `new-validation-structure.json` - Extracted from new version
- `validation-changes.json` - Structured diff
- `validation-changes.md` - Human-readable changes
- `update-summary.md` - Documentation update summary
- `pipeline.log` - Full pipeline execution log

#### Check Workflow Logs

For detailed debugging:
1. Click on the workflow run
2. Click on the "Update Recipe Documentation" job
3. Expand each step to see detailed logs
4. Look for error messages or unexpected behavior

## Troubleshooting

### Workflow doesn't appear in Actions tab

- Verify workflow file is in `.github/workflows/`
- Check file has `.yml` or `.yaml` extension
- Ensure GitHub Actions is enabled in fork

### "No changes detected" when expecting changes

- Check artifact `validation-changes.json` to see what was compared
- Verify versions exist: `git tag | grep v1.15.0`
- Review extraction script logs

### goose CLI installation fails

- Workflow installs from current repository
- Ensure `crates/goose-cli` builds successfully
- Check Rust toolchain installation

### PR creation fails

- Verify workflow has required permissions
- Check branch name doesn't already exist
- Review workflow logs for error messages

## Production Deployment

Once testing is complete:

1. **Merge automation to main**: Create PR for `documentation/automation/recipe-schema-tracking/`
2. **Merge baseline docs**: Create PR for revised `recipe-reference.md`
3. **Merge workflow**: Create PR for `.github/workflows/docs-update-recipe-ref.yml`
4. **Enable release trigger**: Uncomment `release:` section in workflow

After deployment, the workflow will automatically:
- Trigger on new releases
- Compare with previous release
- Create PR if changes detected
- Notify team for review

## Related Documentation

- [Recipe Schema Tracking README](./README.md) - Automation details
- [Recipe Reference Guide](../../docs/guides/recipes/recipe-reference.md) - Target documentation
- [Automation Overview](../README.md) - All automation projects
