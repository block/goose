# Analyze Tool Effectiveness Experiment

Compares agent performance on codebase exploration tasks across three conditions:
1. **with-analyze** - Full `analyze` tool with tree-sitter semantic analysis
2. **with-map** - Lightweight `map` tool (line counts only) + `rg --heading -n`
3. **without-analyze** - Baseline with just shell and text_editor

## Hypothesis

Specialized exploration tools enable more efficient codebase exploration. We're testing whether:
- The full `analyze` tool (semantic analysis, call graphs) justifies its complexity
- A simpler `map` + `rg` approach achieves similar efficiency
- Either beats the baseline of shell commands alone

## Metrics

1. **Token usage** - Total input/output tokens consumed
2. **Tool call count** - Number of tool invocations
3. **Answer quality** - Correctness and completeness (manual evaluation)

## Test Questions

| # | Category | Question |
|---|----------|----------|
| 1 | Orientation | What are the main crates in this project and what does each do? |
| 2 | Symbol lookup | Where is the Agent trait defined and what methods does it have? |
| 3 | Relationship | What calls the complete method on providers? |
| 4 | Architecture | How does a message flow from CLI input to LLM response? |

## Running

```bash
# Run all questions (all three conditions)
./run-experiment.sh

# Run a specific question
./run-experiment.sh 1   # Just question 1

# Analyze results
./analyze-results.sh
```

## Recipes

| Recipe | Extension | Key Tools |
|--------|-----------|-----------|
| `with-analyze.yaml` | developer | analyze, shell, text_editor |
| `with-map.yaml` | develop | map, shell, file_write, file_edit |
| `without-analyze.yaml` | developer | shell, text_editor (no analyze) |

## Results

Results are saved to `./results/` with naming convention:
```
{timestamp}_q{question_num}_{condition}.json
```

Each JSON file contains:
- `messages` - Full conversation history
- `metadata.total_tokens` - Total tokens used
- `metadata.input_tokens` - Input tokens
- `metadata.output_tokens` - Output tokens
- `metadata.status` - Completion status
