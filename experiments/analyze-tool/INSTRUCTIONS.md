# Running the Analyze Tool Experiment

## Prerequisites

The code change for JSON token output is staged but not committed. Build first:

```bash
cd /Users/baxen/Development/goose
cargo build -p goose-cli
```

## Running Experiments

### Option 1: Run All Questions (recommended)
```bash
./experiments/analyze-tool/run-experiment.sh
```
This runs 4 questions × 2 conditions = 8 total runs. Takes ~15-20 minutes.

### Option 2: Run Single Question
```bash
./experiments/analyze-tool/run-experiment.sh 1   # Question 1 only
./experiments/analyze-tool/run-experiment.sh 2   # Question 2 only
```

## Results

Results are saved to `experiments/analyze-tool/results/` with format:
```
{timestamp}_q{N}_{condition}.json
```

Example:
```
20251228_141500_q1_with-analyze.json
20251228_141500_q1_without-analyze.json
```

## Quick Analysis

After running, use:
```bash
./experiments/analyze-tool/analyze-results.sh
```

Or manually check a result:
```bash
cat experiments/analyze-tool/results/*_q1_*.json | jq '.metadata'
```

## Coming Back to Review

When you return to goose, say something like:

> "Let's review the analyze tool experiment results in experiments/analyze-tool/results/"

I'll analyze:
1. Token usage comparison (with vs without analyze)
2. Tool call patterns
3. Answer quality (manual review)
