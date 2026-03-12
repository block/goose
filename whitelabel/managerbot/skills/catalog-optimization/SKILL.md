---
name: Catalog Optimization
description: >
  Prioritized recommendations to improve catalog images, pricing, and categories.
  Use when the user asks how to improve their catalog, optimize listings, organize
  categories, review pricing, or wants actionable next steps to grow sales.
  For a diagnostic audit of what's broken, see catalog-health-check instead.
---

# Catalog Optimization

Produce prioritized recommendations weighted by business impact.

## Step 1: Fetch data

```bash
square catalog list --types ITEM
square catalog list --types CATEGORY
```

Paginate both fully.

Optionally try sales data for revenue weighting:
```bash
square reporting meta
```
If reporting is available, query top items by revenue to prioritize recommendations for high-selling items.

## Step 2: Analyze and recommend

**Images** — which items are missing images? Prioritize by sales volume if available. Items customers see most should have images first.

**Pricing** — are prices consistent within categories? Any items priced far above/below their category peers? Any variations missing prices entirely?

**Categories** — are items well-organized? Too many uncategorized items? Categories that are too broad (50+ items) or too narrow (1 item)?

## Step 3: Report

Structure as:
- Quick wins (high impact, low effort) — e.g. "add images to your 3 best-selling items"
- Strategic improvements (high impact, higher effort) — e.g. "reorganize your 'Food' category into subcategories"
- Maintenance (lower urgency) — e.g. "5 items have no SKU"

Name specific items. If sales data is unavailable, say so and prioritize by completeness instead.
