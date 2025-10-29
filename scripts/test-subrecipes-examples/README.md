# Subrecipe CI Test

Fast, deterministic integration tests for subrecipe functionality using file-based operations.

## Purpose

This test suite validates goose's subrecipe orchestration without relying on slow network calls. It uses local file I/O operations to test the same functionality as the original weather API test, but runs much faster in CI.

## What It Tests

- ✅ Subrecipe invocation and orchestration
- ✅ Sequential vs parallel execution modes
- ✅ Session and no-session modes
- ✅ Parameter passing between parent and subrecipes
- ✅ Subagent spawning and lifecycle

## Test Structure

### Parent Recipes
- **project_analyzer.yaml** - Sequential execution of subrecipes
- **project_analyzer_parallel.yaml** - Parallel execution of subrecipes

### Subrecipes
- **file_stats.yaml** - Analyzes file statistics (counts, sizes, lines)
- **code_patterns.yaml** - Searches for code patterns (TODOs, functions, imports)

## Running the Tests

```bash
# From the goose root directory
./scripts/test_subrecipes.sh
```

## CI Integration

Add to `.github/workflows/pr-smoke-test.yml`:

```yaml
- name: Test Subrecipes
  run: ./scripts/test_subrecipes.sh
```

## Performance

- **Original test**: ~30-60 seconds (network API calls)
- **This test**: ~5-10 seconds (local file operations)

## Advantages Over Network-Based Tests

1. **Fast**: No network latency or API rate limits
2. **Deterministic**: Same results every run
3. **Reliable**: No external dependencies or service outages
4. **Isolated**: Works offline and in restricted networks
5. **Comprehensive**: Still tests all core subrecipe functionality
