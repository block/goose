---
name: Catalog Health Check
description: >
  Diagnostic audit of a merchant's product catalog — missing fields, duplicates,
  data quality. Use when the user asks about catalog health, data quality,
  incomplete listings, duplicate items, missing images/descriptions/prices/SKUs,
  or wants an audit of their catalog.
---

# Catalog Health Check

Diagnose data quality issues across four areas: completeness, duplicates, pricing, and categories.

## Step 1: Fetch catalog data

```bash
square catalog list --types ITEM
```

Paginate until no cursor remains. Also fetch categories:

```bash
square catalog list --types CATEGORY
```

## Step 2: Analyze

For each item, check:

**Completeness** — does item_data have:
- `description` (at least 10 chars)?
- `image_ids` (non-empty)?
- categories (via `item_data.categories[].id`)?
- Each variation: has `price_money`? has `sku`?

**Duplicates** — are any item names identical (case-insensitive)? Are any SKUs shared across variations?

**Pricing** — any variations with fixed pricing but no `price_money`? Any obvious outliers within a category?

**Categories** — any items with no category? Any empty categories (no items assigned)? Any overcrowded categories (50+ items)?

## Step 3: Report

Summarize findings. Name the specific items with issues — merchants act on names, not counts.

Structure: overall picture, then high priority (affects customers — missing images, missing prices), medium priority (missing categories, possible duplicates), low priority (missing SKUs, empty categories).

Keep it concise. Don't dump raw JSON.

## Tips

- For small catalogs (<10 items), keep it brief.
- Empty catalog → just say so, don't produce a report of zeros.
- If the merchant asked about one specific area (e.g. "do I have duplicates?"), only check that.
