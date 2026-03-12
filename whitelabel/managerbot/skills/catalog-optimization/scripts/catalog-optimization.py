#!/usr/bin/env python3
"""Catalog Optimization Guide — analysis engine.

Reads consolidated catalog + sales JSON from stdin (produced by gather-catalog.sh)
and generates a structured optimization report across images, pricing, and categories.

Usage:
  gather-catalog.sh --focus-area full | python catalog-optimization.py

Output: JSON optimization report on stdout.
"""

import json
import re
import sys
from collections import defaultdict


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

SCORE_VALUES = {"looks_good": 100, "ok": 60, "needs_improvement": 20}


def category_ids(item_data):
    ids = []
    for cat in item_data.get("categories") or []:
        cid = cat.get("id")
        if cid:
            ids.append(cid)
    if not ids and item_data.get("category_id"):
        ids.append(item_data["category_id"])
    if not ids:
        reporting = item_data.get("reporting_category") or {}
        if reporting.get("id"):
            ids.append(reporting["id"])
    return ids


def variation_prices(item_data):
    prices = []
    for variation in item_data.get("variations") or []:
        vd = variation.get("item_variation_data") or {}
        if vd.get("pricing_type") == "VARIABLE_PRICING":
            continue
        pm = vd.get("price_money") or {}
        amount = pm.get("amount")
        if isinstance(amount, int) and amount > 0:
            prices.append(amount)
    return prices


def representative_price(item_data):
    prices = sorted(variation_prices(item_data))
    if not prices:
        return None
    return prices[len(prices) // 2]


def normalize_name(name):
    return re.sub(r"[^a-z0-9]+", "", (name or "").lower())


def format_price(cents):
    return f"${cents / 100:.2f}"


def score_ratio(issue_ratio, good_max, ok_max):
    if issue_ratio <= good_max:
        return "looks_good"
    if issue_ratio <= ok_max:
        return "ok"
    return "needs_improvement"


def truncate(items, n):
    shown = items[:n]
    rest = len(items) - n
    if rest > 0:
        shown.append(f"and {rest} more")
    return ", ".join(shown)


# ---------------------------------------------------------------------------
# Build internal catalog representation from raw API objects
# ---------------------------------------------------------------------------

def build_catalog(item_objects, category_objects):
    categories = {}
    for cat in category_objects:
        categories[cat["id"]] = (cat.get("category_data") or {}).get("name", cat["id"])

    items = []
    for item in item_objects:
        item_data = item.get("item_data") or {}
        prices = variation_prices(item_data)
        items.append(
            {
                "id": item.get("id", ""),
                "name": item_data.get("name", "?"),
                "description": item_data.get("description") or "",
                "has_image": bool(item_data.get("image_ids")),
                "category_ids": category_ids(item_data),
                "prices": prices,
                "price_cents": representative_price(item_data),
                "variation_count": len(item_data.get("variations") or []),
                "sales_net": 0.0,
                "sales_qty": 0.0,
            }
        )

    return items, categories


def attach_sales(items, sales_signal):
    by_id = {}
    by_name = defaultdict(float)
    by_name_qty = defaultdict(float)

    for row in sales_signal.get("rows") or []:
        rid = row.get("item_id") or ""
        if rid:
            by_id[rid] = row
        rname = normalize_name(row.get("item_name", ""))
        if rname:
            by_name[rname] += row.get("net_sales", 0.0)
            by_name_qty[rname] += row.get("quantity_sold", 0.0)

    matched = 0
    nonzero = 0
    for item in items:
        row = by_id.get(item["id"])
        if row:
            item["sales_net"] = row.get("net_sales", 0.0)
            item["sales_qty"] = row.get("quantity_sold", 0.0)
            matched += 1
        else:
            key = normalize_name(item["name"])
            if key in by_name:
                item["sales_net"] = by_name[key]
                item["sales_qty"] = by_name_qty[key]
                matched += 1
        if item["sales_net"] > 0:
            nonzero += 1

    if matched == 0:
        sales_signal["status"] = "unavailable"
        sales_signal["note"] = sales_signal.get("note", "") + "; no catalog item matches in sales data"
    elif nonzero == 0:
        sales_signal["status"] = "zero_activity"
        sales_signal["note"] = sales_signal.get("note", "") + "; matched items but all sales are zero"
    else:
        sales_signal["status"] = "available"
        sales_signal["note"] = sales_signal.get("note", "") + f"; matched {matched} item(s)"


# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------

def compute_stats(items, categories):
    total_variations = sum(i["variation_count"] for i in items)
    missing_images = [i["name"] for i in items if not i["has_image"]]
    missing_desc = [i["name"] for i in items if len((i["description"] or "").strip()) == 0]
    uncategorized = [i["name"] for i in items if not i["category_ids"]]
    complete_items = 0
    for i in items:
        if i["has_image"] and i["category_ids"] and len((i["description"] or "").strip()) > 0 and i["price_cents"] is not None:
            complete_items += 1
    complete_pct = round((complete_items / len(items)) * 100) if items else 100

    return {
        "total_items": len(items),
        "total_variations": total_variations,
        "total_categories": len(categories),
        "catalog_completeness_percent": complete_pct,
        "items_missing_images_count": len(missing_images),
        "items_missing_descriptions_count": len(missing_desc),
        "uncategorized_items_count": len(uncategorized),
        "items_missing_images": "; ".join(missing_images),
        "items_missing_descriptions": "; ".join(missing_desc),
        "uncategorized_items": "; ".join(uncategorized),
    }


def category_index(items):
    by_category = defaultdict(list)
    for item in items:
        for cid in item["category_ids"]:
            by_category[cid].append(item)
    return by_category


# ---------------------------------------------------------------------------
# Images
# ---------------------------------------------------------------------------

def analyze_images(items, categories, by_category):
    missing = [i for i in items if not i["has_image"]]
    prioritized = sorted(
        missing,
        key=lambda i: (i["sales_net"], i["price_cents"] or 0),
        reverse=True,
    )
    top_no_image = []
    for item in prioritized[:10]:
        if item["sales_net"] > 0:
            top_no_image.append(f"{item['name']} (${item['sales_net']:.2f} sales)")
        else:
            top_no_image.append(item["name"])

    low_coverage = []
    for cid, cat_items in by_category.items():
        with_images = sum(1 for i in cat_items if i["has_image"])
        coverage = round((with_images / len(cat_items)) * 100) if cat_items else 100
        if coverage < 50:
            low_coverage.append(f"{categories.get(cid, cid)}({coverage}%)")

    score = score_ratio((len(missing) / len(items)) if items else 0, 0.10, 0.30)

    high = []
    if top_no_image:
        high.append(f"Add images to: {truncate(top_no_image, 8)}")
    medium = []
    if low_coverage:
        medium.append(f"Raise category image coverage: {truncate(low_coverage, 6)}")

    return {
        "images_score": score,
        "top_sellers_without_images": "; ".join(top_no_image),
        "low_image_coverage_categories": "; ".join(low_coverage),
        "high_actions": high,
        "medium_actions": medium,
        "low_actions": [],
    }


# ---------------------------------------------------------------------------
# Pricing
# ---------------------------------------------------------------------------

def analyze_pricing(items, categories, by_category):
    priced = [i for i in items if i["price_cents"] is not None]
    outliers = []
    inconsistent_endings = []
    variation_gaps = []

    for cid, cat_items in by_category.items():
        prices = sorted([i["price_cents"] for i in cat_items if i["price_cents"] is not None])
        if len(prices) >= 4:
            n = len(prices)
            q1 = prices[n // 4]
            q3 = prices[(3 * n) // 4]
            iqr = q3 - q1
            low = q1 - 1.5 * iqr
            high = q3 + 1.5 * iqr
            med = prices[n // 2]
            for item in cat_items:
                p = item["price_cents"]
                if p is None:
                    continue
                if p < low or p > high:
                    outliers.append(
                        f"{item['name']}({format_price(p)} vs {categories.get(cid, cid)} median {format_price(med)})"
                    )

        if len(prices) >= 5:
            endings = defaultdict(int)
            for p in prices:
                endings[p % 100] += 1
            dominant_share = max(endings.values()) / len(prices)
            if dominant_share < 0.6:
                inconsistent_endings.append(f"{categories.get(cid, cid)}({round(dominant_share * 100)}% dominant)")

    for item in items:
        p = sorted(item["prices"])
        if len(p) >= 3 and p[0] > 0 and p[-1] >= (2 * p[0]):
            variation_gaps.append(f"{item['name']}({format_price(p[0])} to {format_price(p[-1])})")

    issue_ratio = (len(outliers) + len(inconsistent_endings) + len(variation_gaps)) / max(1, len(priced))
    score = score_ratio(issue_ratio, 0.08, 0.20)

    high = []
    if outliers:
        high.append(f"Review price outliers: {truncate(outliers, 8)}")
    medium = []
    if inconsistent_endings:
        medium.append(f"Standardize price endings: {truncate(inconsistent_endings, 6)}")
    low = []
    if variation_gaps:
        low.append(f"Review variation pricing gaps: {truncate(variation_gaps, 6)}")

    return {
        "pricing_score": score,
        "price_outliers": "; ".join(outliers),
        "inconsistent_price_endings_categories": "; ".join(inconsistent_endings),
        "variation_pricing_issues": "; ".join(variation_gaps),
        "high_actions": high,
        "medium_actions": medium,
        "low_actions": low,
    }


# ---------------------------------------------------------------------------
# Categories
# ---------------------------------------------------------------------------

def analyze_categories(items, categories, by_category):
    uncategorized = [i["name"] for i in items if not i["category_ids"]]
    empty = []
    sparse = []
    overcrowded = []

    for cid, name in categories.items():
        count = len(by_category.get(cid) or [])
        if count == 0:
            empty.append(name)
        elif count < 3:
            sparse.append(f"{name}({count})")
        elif count > 30:
            overcrowded.append(f"{name}({count})")

    issue_ratio = (len(uncategorized) + len(empty) + len(sparse) + len(overcrowded)) / max(1, len(items) + len(categories))
    score = score_ratio(issue_ratio, 0.08, 0.18)

    high = []
    if uncategorized:
        high.append(f"Assign categories to: {truncate(uncategorized, 10)}")
    medium = []
    if overcrowded:
        medium.append(f"Split large categories: {truncate(overcrowded, 6)}")
    low = []
    if sparse:
        low.append(f"Merge sparse categories: {truncate(sparse, 8)}")
    if empty:
        low.append(f"Remove empty categories: {truncate(empty, 8)}")

    return {
        "categories_score": score,
        "empty_categories": "; ".join(empty),
        "sparse_categories": "; ".join(sparse),
        "overcrowded_categories": "; ".join(overcrowded),
        "high_actions": high,
        "medium_actions": medium,
        "low_actions": low,
    }


# ---------------------------------------------------------------------------
# Category summary table
# ---------------------------------------------------------------------------

def category_summary(items, categories, by_category):
    rows = []
    for cid, name in categories.items():
        cat_items = by_category.get(cid) or []
        if not cat_items:
            continue
        count = len(cat_items)
        image_count = sum(1 for i in cat_items if i["has_image"])
        image_pct = round((image_count / count) * 100)
        prices = [i["price_cents"] for i in cat_items if i["price_cents"] is not None]
        avg_price = format_price(round(sum(prices) / len(prices))) if prices else "n/a"
        recs = []
        if image_pct < 50:
            recs.append("add images")
        if count < 3:
            recs.append("consider merge")
        if count > 30:
            recs.append("split category")
        if not recs:
            recs.append("maintain")
        rows.append(
            {
                "category": name,
                "items": count,
                "images_percent": image_pct,
                "avg_price": avg_price,
                "recommendations": ", ".join(recs),
            }
        )
    rows.sort(key=lambda r: r["items"], reverse=True)
    return rows[:30]


# ---------------------------------------------------------------------------
# Overall score
# ---------------------------------------------------------------------------

def score_overall(images_score, pricing_score, categories_score):
    i = SCORE_VALUES[images_score]
    p = SCORE_VALUES[pricing_score]
    c = SCORE_VALUES[categories_score]
    return round((i * 0.4) + (p * 0.3) + (c * 0.3))


def join_actions(actions):
    if not actions:
        return ""
    return "; ".join(actions)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    try:
        raw = json.load(sys.stdin)
    except json.JSONDecodeError as e:
        print(json.dumps({"error": f"Invalid JSON on stdin: {e}"}), file=sys.stderr)
        sys.exit(1)

    focus = raw.get("focus_area", "full")
    item_objects = [o for o in (raw.get("items") or []) if o.get("type") == "ITEM"]
    category_objects = [o for o in (raw.get("categories") or []) if o.get("type") == "CATEGORY"]
    sales_signal = raw.get("sales_signal") or {"available": False, "status": "unavailable", "rows": []}

    try:
        items, categories = build_catalog(item_objects, category_objects)
        by_category = category_index(items)
        stats = compute_stats(items, categories)

        attach_sales(items, sales_signal)

        images = analyze_images(items, categories, by_category)
        pricing = analyze_pricing(items, categories, by_category)
        categories_report = analyze_categories(items, categories, by_category)
        cat_summary = category_summary(items, categories, by_category)

        if focus == "images":
            data = {
                **stats,
                "sales_signal_status": sales_signal["status"],
                "sales_signal_note": sales_signal.get("note", ""),
                **images,
                "category_summary": cat_summary,
            }
        elif focus == "pricing":
            data = {
                **stats,
                "sales_signal_status": sales_signal["status"],
                "sales_signal_note": sales_signal.get("note", ""),
                **pricing,
                "category_summary": cat_summary,
            }
        elif focus == "categories":
            data = {
                **stats,
                "sales_signal_status": sales_signal["status"],
                "sales_signal_note": sales_signal.get("note", ""),
                **categories_report,
                "category_summary": cat_summary,
            }
        else:
            overall = score_overall(
                images["images_score"],
                pricing["pricing_score"],
                categories_report["categories_score"],
            )
            quick_wins = images["high_actions"] + pricing["high_actions"] + categories_report["high_actions"]
            strategic = images["medium_actions"] + pricing["medium_actions"] + categories_report["medium_actions"]
            maintenance = images["low_actions"] + pricing["low_actions"] + categories_report["low_actions"]

            data = {
                **stats,
                "optimization_score": overall,
                "estimated_optimization_potential": len(quick_wins) + len(strategic) + len(maintenance),
                "sales_signal_status": sales_signal["status"],
                "sales_signal_note": sales_signal.get("note", ""),
                "images_score": images["images_score"],
                "pricing_score": pricing["pricing_score"],
                "categories_score": categories_report["categories_score"],
                "top_sellers_without_images": images["top_sellers_without_images"],
                "low_image_coverage_categories": images["low_image_coverage_categories"],
                "price_outliers": pricing["price_outliers"],
                "inconsistent_price_endings_categories": pricing["inconsistent_price_endings_categories"],
                "variation_pricing_issues": pricing["variation_pricing_issues"],
                "empty_categories": categories_report["empty_categories"],
                "sparse_categories": categories_report["sparse_categories"],
                "overcrowded_categories": categories_report["overcrowded_categories"],
                "quick_wins": join_actions(quick_wins),
                "strategic_improvements": join_actions(strategic),
                "maintenance_items": join_actions(maintenance),
                "category_summary": cat_summary,
            }

        print(json.dumps({"focus_area": focus, "data": data}, indent=2))

    except Exception as exc:
        print(json.dumps({"error": str(exc)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
