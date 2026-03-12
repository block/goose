---
name: Catalog Health Check
description: >
  Run a diagnostic audit of a merchant's product catalog to find missing fields, duplicates,
  and data quality problems. Use this skill whenever the user asks about catalog health,
  data quality, incomplete listings, duplicate items, missing images/descriptions/prices/SKUs,
  pricing inconsistencies, category organization, or wants an audit of their catalog — even if
  they don't explicitly say "health check." For strategic optimization recommendations weighted
  by sales data, see catalog-optimization instead.
---

# Catalog Health Check

Diagnose data quality issues across four dimensions: completeness, duplicates, pricing,
and category organization. The goal is to surface what's broken or missing so the merchant
can fix it — this is a diagnostic tool, not a strategic advisor.

> **Script paths:** Replace `$SKILL_DIR` below with the absolute skill directory path shown in the "Supporting Files" section above (e.g. `/Users/.../catalog-health-check`).

## Step 1: Gather catalog data

Run the data gathering script to fetch all items and categories from the Square API:

```bash
$SKILL_DIR/scripts/gather-catalog.sh --focus-area full
```

For a targeted request, only gather the relevant data:
```bash
$SKILL_DIR/scripts/gather-catalog.sh --focus-area completeness   # items only
$SKILL_DIR/scripts/gather-catalog.sh --focus-area duplicates     # items only
$SKILL_DIR/scripts/gather-catalog.sh --focus-area pricing        # items + categories
$SKILL_DIR/scripts/gather-catalog.sh --focus-area categories     # items + categories
```

This orchestrates:
- `square catalog list --types ITEM` (with pagination) for all catalog items
- `square catalog list --types CATEGORY` (with pagination) when needed

The script outputs consolidated JSON to stdout.

## Step 2: Analyze the data

Pipe the gathered data into the analysis script:

```bash
$SKILL_DIR/scripts/gather-catalog.sh --focus-area full \
  | python3 $SKILL_DIR/scripts/catalog-health-check.py
```

The analysis script reads JSON from stdin, computes deterministic health scores,
and outputs a structured JSON report. It does not make any API calls.

## Step 3: Present the report

Translate the JSON into a clean report for the merchant. Use this structure:

**Catalog Health Report**

**Overall Score: XX/100**

| Metric | Value |
|--------|-------|
| Total Items | XXX |
| Total Variations | XXX |
| Complete Items | XX% |
| Items Missing Images | XX |
| Items Missing Descriptions | XX |
| Items Missing Categories | XX |
| Variations Missing Prices | XX |
| Variations Missing SKUs | XX |

| Area | Score |
|------|-------|
| Completeness | looks_good / ok / needs_improvement |
| Duplicates | looks_good / ok / needs_improvement |
| Pricing | looks_good / ok / needs_improvement |
| Categories | looks_good / ok / needs_improvement |

**High Priority** (affects customer experience)
[items needing images or prices, by name; exact duplicate groups to merge]

**Medium Priority** (review and decide)
[fuzzy near-duplicates to review; items needing categories]

**Low Priority** (housekeeping)
[items needing SKUs, duplicate SKUs, empty or redundant categories]

## Step 4: Offer follow-up

After presenting the report, offer to look up specific flagged items or help fix issues.

## Scoring reference

Each dimension gets a qualitative score: `looks_good` (100), `ok` (60), or `needs_improvement` (20).
- **Completeness**: based on % of items with all fields filled (>=80% = looks_good, >=50% = ok)
- **Duplicates**: based on count of exact + fuzzy duplicate groups + duplicate SKUs (0 = looks_good, 1-5 = ok, >5 = needs_improvement)
- **Pricing**: based on count of zero-price items + IQR-based outliers (0 = looks_good, 1-5 = ok, >5 = needs_improvement)
- **Categories**: based on how many issues exist: empty, overcrowded (>50), single-item, uncategorized (0 = looks_good, 1-3 = ok, >3 = needs_improvement)

Overall = Completeness 40% + Duplicates 20% + Pricing 20% + Categories 20%.

## Edge cases

- **Empty catalog**: Tell the merchant their catalog is empty — don't post a report full of zeros.
- **CLI errors**: Explain plainly (e.g., "Couldn't connect to your catalog").
- **Very small catalogs** (<5 items): Keep recommendations brief. Don't flag statistical outliers with too few data points.

## Tips

- Name the actual items with issues — merchants act on names, not counts.
- For a targeted request (e.g., "do I have duplicates?"), only run that focus area.
- Present one final report, not intermediate progress updates.
