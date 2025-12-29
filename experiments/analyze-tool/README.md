# Analyze Tool Effectiveness Experiment

Compares agent performance on codebase exploration tasks **with** and **without** the `analyze` tool.

## Hypothesis

The `analyze` tool enables more efficient codebase exploration by providing structured code intelligence (file trees, function signatures, call graphs) without requiring the agent to read full file contents.

## Metrics

1. **Token usage** - Total input/output tokens consumed
2. **Answer quality** - Correctness and completeness (manual evaluation)
3. **Tool call patterns** - How the agent explores the codebase

## Test Questions

| # | Category | Question |
|---|----------|----------|
| 1 | Orientation | What are the main crates in this project and what does each do? |
| 2 | Symbol lookup | Where is the Agent trait defined and what methods does it have? |
| 3 | Relationship | What calls the complete method on providers? |
| 4 | Architecture | How does a message flow from CLI input to LLM response? |

## Running

```bash
# Run all questions (both conditions)
./run-experiment.sh

# Run a specific question
./run-experiment.sh 1   # Just question 1

# Analyze results
./analyze-results.sh
```

## Recipes

- `with-analyze.yaml` - Developer extension with all tools (including analyze)
- `without-analyze.yaml` - Developer extension without analyze tool

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
