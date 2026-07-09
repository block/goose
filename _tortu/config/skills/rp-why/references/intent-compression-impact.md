# Intent Compression: Impact on Metrics

## How Compression Affects Each Report

### Baseline (/rp-why init)
- Compression % is calculated across all historical sessions
- Adjusted DOK becomes the primary DOK number
- Growth targets include a compression target (typically current + 5%, min 5%)

### Current (/rp-why current)
- Compression % for this session specifically
- Adjusted DOK shown as primary, raw available for comparison
- DOK lift (adjusted - raw) quantifies the compression effect

### Compare (/rp-why compare)
- Compression delta between baseline and current
- If compression was 0% at baseline and is now >0%, shows "emerged"
- Trajectory interpretation accounts for compression (execution-heavy sessions may show low raw DOK but high compression)

### Overall (/rp-why overall)
- Compression % tracked per phase
- Phase transitions often correlate with compression emergence
- Floor tracking uses adjusted DOK

## Scoring Adjustment

```
For each prompt in a session:
  1. Classify DOK using keyword patterns (raw DOK)
  2. Check compression signals:
     - Word count <= 8 AND prompt index > 2
     - Matches known compression pattern OR (word count <= 4 AND prompt index > 5)
  3. If compressed: adjusted_dok = min(raw_dok + 1, 4)
  4. Otherwise: adjusted_dok = raw_dok

Session metrics:
  - dok_raw = mean(all raw DOK scores)
  - dok_adjusted = mean(all adjusted DOK scores)
  - dok_lift = dok_adjusted - dok_raw
  - compression_pct = compressed_count / total_prompts * 100
```

## Interaction with TM and ADT

- High compression + high TM tier = strong Frontier signal
- High compression + low TM tier = possible Thinking Ahead (user has trust patterns but tools are basic)
- Low compression + high TM tier = early in the relationship (tools are sophisticated but trust hasn't built yet)

## Open Questions

1. Should cross-session context factor in? (e.g., "3rd session today on the same strategic doc")
2. Should compression detection weight response complexity? (high response:prompt token ratio as additional signal)
3. Optimal compression detection thresholds may vary by user - should they be calibrated from baseline data?

**Source:** Dakota Fabro (2026), rp-why longitudinal dataset
