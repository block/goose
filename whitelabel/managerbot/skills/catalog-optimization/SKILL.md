---
name: Catalog Optimization Guide
description: >
  Generate prioritized, sales-weighted recommendations to improve catalog images, pricing
  strategy, and category organization. Use this skill whenever the user asks how to improve
  their catalog, optimize product listings, organize categories, review pricing strategy,
  identify which items need better images, or wants actionable next steps to grow sales —
  even if they don't say "optimize." This skill enriches analysis with sales data when
  available, so it's best for strategic decisions. For a diagnostic audit of missing fields
  and duplicates, see catalog-health-check instead.
---

# Catalog Optimization Guide

Produce prioritized recommendations across images, pricing, and category organization.
Unlike the health check (which diagnoses what's broken), this skill focuses on what to
improve next for the biggest business impact — weighted by sales data when available.

> **Script paths:** Replace `$SKILL_DIR` below with the absolute skill directory path shown in the "Supporting Files" section above (e.g. `/Users/.../catalog-optimization`).

## Step 1: Gather catalog and sales data

Run the data gathering script to fetch catalog items, categories, and sales signal:

```bash
$SKILL_DIR/scripts/gather-catalog.sh --focus-area full
```

For targeted requests:
```bash
$SKILL_DIR/scripts/gather-catalog.sh --focus-area images      # items + categories + sales
$SKILL_DIR/scripts/gather-catalog.sh --focus-area pricing     # items + categories (no sales)
$SKILL_DIR/scripts/gather-catalog.sh --focus-area categories  # items + categories (no sales)
```

This orchestrates:
- `square catalog list --types ITEM` (with pagination) for all catalog items
- `square catalog list --types CATEGORY` (with pagination) for all categories
- `square reporting meta` + `square reporting query` for best-effort sales data (images/full only)

The script outputs consolidated JSON to stdout. Sales data is only fetched when it
adds value (revenue weighting for image prioritization).

## Step 2: Analyze the data

Pipe the gathered data into the analysis script:

```bash
$SKILL_DIR/scripts/gather-catalog.sh --focus-area full \
  | python3 $SKILL_DIR/scripts/catalog-optimization.py
```

The analysis script reads JSON from stdin, computes optimization scores and
recommendations, and outputs a structured JSON report. It does not make any API calls.

## Step 3: Present the report

Translate the JSON into a clean report for the merchant:

**Catalog Optimization Guide**

**Optimization Score: XX/100**
**Sales Signal: available / zero_activity / unavailable**

| Metric | Value |
|--------|-------|
| Total Items | XXX |
| Total Variations | XXX |
| Total Categories | XXX |
| Catalog Completeness | XX% |
| Items Missing Images | XX |
| Items Missing Descriptions | XX |
| Uncategorized Items | XX |

| Area | Score |
|------|-------|
| Images | looks_good / ok / needs_improvement |
| Pricing | looks_good / ok / needs_improvement |
| Categories | looks_good / ok / needs_improvement |

**Quick Wins** (high impact, lower effort)
[from quick_wins]

**Strategic Improvements** (high impact, higher effort)
[from strategic_improvements]

**Maintenance Items** (lower urgency)
[from maintenance_items]

| Category | Items | Images | Avg Price | Recommendations |
|----------|-------|--------|-----------|-----------------|
[from category_summary]

## Step 4: Offer follow-up

After presenting the report, offer to inspect specific flagged items or categories,
or help implement the top recommendation.

## Scoring reference

Each area gets a qualitative score: `looks_good` (100), `ok` (60), or `needs_improvement` (20).
- **Images**: based on ratio of items missing images (<=10% = looks_good, <=30% = ok)
- **Pricing**: based on ratio of outliers + inconsistent endings + variation gaps (<=8% = looks_good, <=20% = ok)
- **Categories**: based on ratio of issues (uncategorized + empty + sparse + overcrowded) (<=8% = looks_good, <=18% = ok)

Overall = Images 40% + Pricing 30% + Categories 30%.

## Edge cases

- **Empty catalog**: Tell the merchant their catalog is empty.
- **CLI errors**: Explain plainly.
- **Very small catalogs** (<5 items): Keep recommendations brief and practical.
- **Sales data unavailable**: The script gracefully degrades. Don't claim items are "top sellers" without sales data.

## Tips

- For small catalogs (<10 items), keep recommendations pragmatic.
- If sales signal is unavailable, do not make revenue-weighted claims.
- Name concrete item/category names, not just counts.
- Present one final report, not intermediate updates.
