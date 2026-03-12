#!/usr/bin/env python3
"""Catalog Health Check — deterministic analysis of a Square merchant catalog.

Reads consolidated catalog JSON from stdin (produced by gather-catalog.sh)
and computes health scores across four dimensions: completeness, duplicates,
pricing, and category organization.

Usage:
  gather-catalog.sh --focus-area full | python catalog-health-check.py

Output: JSON health report on stdout.
"""

import json
import sys
from collections import defaultdict
from difflib import SequenceMatcher


def token_sort_ratio(s1, s2):
    """Replicates thefuzz.fuzz.token_sort_ratio using only stdlib.

    Tokenizes both strings, sorts the tokens, then computes a similarity
    ratio (0-100) using SequenceMatcher.
    """
    t1 = " ".join(sorted(s1.split()))
    t2 = " ".join(sorted(s2.split()))
    return int(SequenceMatcher(None, t1, t2).ratio() * 100)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

SCORE_VALUES = {"looks_good": 100, "ok": 60, "needs_improvement": 20}


def score_from_thresholds(issue_count, high_threshold):
    if issue_count > high_threshold:
        return "needs_improvement"
    if issue_count >= 1:
        return "ok"
    return "looks_good"


def format_price(cents):
    return f"${cents / 100:.2f}"


def category_ids(item_data):
    """Extract category IDs from item_data, handling all API formats."""
    ids = []
    for cat in item_data.get("categories") or []:
        if cat.get("id"):
            ids.append(cat["id"])
    if not ids and item_data.get("category_id"):
        ids.append(item_data["category_id"])
    if not ids:
        rc = item_data.get("reporting_category") or {}
        if rc.get("id"):
            ids.append(rc["id"])
    return ids


def truncate_list(items, n, fmt=None):
    """Format a list with 'and N more' suffix when truncated."""
    shown = items[:n]
    rest = len(items) - n
    result = [fmt(i) if fmt else i for i in shown]
    if rest > 0:
        result.append(f"and {rest} more")
    return ", ".join(result)


# ---------------------------------------------------------------------------
# Deterministic stats (computed once, passed to analyzers)
# ---------------------------------------------------------------------------

def compute_stats(items):
    total_items = len(items)
    total_variations = 0
    missing_desc = 0
    missing_images = 0
    missing_cats = 0
    vars_missing_skus = 0
    vars_missing_prices = 0
    complete_items = 0

    for item in items:
        d = item.get("item_data", {})
        variations = d.get("variations") or []
        total_variations += len(variations)

        has_desc = len(d.get("description") or "") >= 10
        has_img = bool(d.get("image_ids"))
        has_cat = bool(category_ids(d))

        if not has_desc:
            missing_desc += 1
        if not has_img:
            missing_images += 1
        if not has_cat:
            missing_cats += 1

        all_vars_ok = True
        for v in variations:
            vd = v.get("item_variation_data", {})
            if vd.get("pricing_type") != "VARIABLE_PRICING" and not vd.get("price_money"):
                vars_missing_prices += 1
                all_vars_ok = False
            if not vd.get("sku"):
                vars_missing_skus += 1
                all_vars_ok = False

        if has_desc and has_img and has_cat and all_vars_ok:
            complete_items += 1

    complete_pct = round(complete_items / total_items * 100) if total_items else 100
    return {
        "total_items": total_items,
        "total_variations": total_variations,
        "complete_items_percent": complete_pct,
        "items_missing_descriptions": missing_desc,
        "items_missing_images": missing_images,
        "items_missing_categories": missing_cats,
        "variations_missing_skus": vars_missing_skus,
        "variations_missing_prices": vars_missing_prices,
    }


# ---------------------------------------------------------------------------
# Completeness
# ---------------------------------------------------------------------------

def analyze_completeness(items, stats):
    pct = stats["complete_items_percent"]
    score = "looks_good" if pct >= 80 else ("ok" if pct >= 50 else "needs_improvement")

    issue_items = []
    for item in items:
        d = item.get("item_data", {})
        missing = []
        if len(d.get("description") or "") < 10:
            missing.append("description")
        if not d.get("image_ids"):
            missing.append("image")
        if not category_ids(d):
            missing.append("category")
        for v in (d.get("variations") or []):
            vd = v.get("item_variation_data", {})
            if vd.get("pricing_type") != "VARIABLE_PRICING" and not vd.get("price_money"):
                missing.append("price")
                break
        for v in (d.get("variations") or []):
            vd = v.get("item_variation_data", {})
            if not vd.get("sku"):
                missing.append("sku")
                break
        if missing:
            issue_items.append((len(missing), d.get("name", "?"), missing))
    issue_items.sort(reverse=True)
    top_issue_items = "; ".join(
        f"{name}(missing:{','.join(m)})" for _, name, m in issue_items[:10]
    )

    needs_images = [i["item_data"]["name"] for i in items if not i.get("item_data", {}).get("image_ids")]
    needs_prices = [
        i["item_data"]["name"] for i in items
        if any(
            v.get("item_variation_data", {}).get("pricing_type") != "VARIABLE_PRICING"
            and not v.get("item_variation_data", {}).get("price_money")
            for v in (i.get("item_data", {}).get("variations") or [])
        )
    ]
    needs_cats = [i["item_data"]["name"] for i in items if not category_ids(i.get("item_data", {}))]
    needs_skus = [
        i["item_data"]["name"] for i in items
        if any(not v.get("item_variation_data", {}).get("sku") for v in (i.get("item_data", {}).get("variations") or []))
    ]

    high = []
    if needs_images:
        high.append(f"Add images to: {truncate_list(needs_images, 10)}")
    if needs_prices:
        high.append(f"Add prices to: {truncate_list(needs_prices, 10)}")
    medium = []
    if needs_cats:
        medium.append(f"Assign categories to: {truncate_list(needs_cats, 10)}")
    low = []
    if needs_skus:
        low.append(f"Add SKUs to: {truncate_list(needs_skus, 10)}")

    return {
        "completeness_score": score,
        "top_issue_items": top_issue_items,
        "high_priority_actions": "; ".join(high),
        "medium_priority_actions": "; ".join(medium),
        "low_priority_actions": "; ".join(low),
        **stats,
    }


# ---------------------------------------------------------------------------
# Duplicates
# ---------------------------------------------------------------------------

def find_fuzzy_duplicates(items, exact_ids, threshold=85):
    """Find near-duplicate item names using fuzzy matching.

    Compares all item pairs and groups those with a token_sort_ratio >= threshold.
    Items already in exact duplicate groups are excluded to avoid redundant results.
    """
    # Build list of (name, item) for items not already flagged as exact duplicates
    candidates = []
    for item in items:
        if item["id"] in exact_ids:
            continue
        name = (item.get("item_data", {}).get("name") or "").strip()
        if name:
            candidates.append((name, item))

    # Union-find to group fuzzy matches
    parent = list(range(len(candidates)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb

    for i in range(len(candidates)):
        for j in range(i + 1, len(candidates)):
            score = token_sort_ratio(candidates[i][0].lower(), candidates[j][0].lower())
            if score >= threshold:
                union(i, j)

    # Collect groups
    groups = defaultdict(list)
    for i in range(len(candidates)):
        groups[find(i)].append(candidates[i])

    return [(items_in_group[0][0], items_in_group) for items_in_group in groups.values() if len(items_in_group) > 1]


def analyze_duplicates(items, stats):
    # Exact name duplicates (case-insensitive)
    name_groups = defaultdict(list)
    for item in items:
        key = (item.get("item_data", {}).get("name") or "").lower().strip()
        name_groups[key].append(item)
    dup_name_groups = [(k, v) for k, v in name_groups.items() if len(v) > 1]

    exact_duplicates = "; ".join(
        f"{v[0]['item_data']['name']}:{','.join(i['id'] for i in v)}"
        for _, v in dup_name_groups
    )

    # IDs already covered by exact matches — skip these for fuzzy
    exact_ids = set()
    for _, group in dup_name_groups:
        for item in group:
            exact_ids.add(item["id"])

    # Fuzzy near-duplicates (e.g. "Iced Latte 16oz" vs "Iced Latte - 16 oz")
    fuzzy_groups = find_fuzzy_duplicates(items, exact_ids)
    fuzzy_duplicates = "; ".join(
        " ~ ".join(name for name, _ in group)
        for _, group in fuzzy_groups
    )

    # Duplicate SKUs across all variations
    sku_to_var_ids = defaultdict(set)
    for item in items:
        for v in (item.get("item_data", {}).get("variations") or []):
            sku = v.get("item_variation_data", {}).get("sku")
            if sku:
                sku_to_var_ids[sku].add(v["id"])
    dup_skus = [(sku, ids) for sku, ids in sku_to_var_ids.items() if len(ids) > 1]

    duplicate_skus = "; ".join(f"{sku}:{','.join(sorted(ids))}" for sku, ids in dup_skus)

    total_groups = len(dup_name_groups) + len(fuzzy_groups) + len(dup_skus)
    score = score_from_thresholds(total_groups, 5)

    high = []
    if dup_name_groups:
        names = [v[0]["item_data"]["name"] for _, v in dup_name_groups]
        high.append(f"Merge {len(dup_name_groups)} exact duplicate group(s): {truncate_list(names, 5)}")
    medium = []
    if fuzzy_groups:
        pairs = [" ~ ".join(name for name, _ in group) for _, group in fuzzy_groups]
        medium.append(f"Review {len(fuzzy_groups)} possible duplicate(s): {truncate_list(pairs, 5)}")
    low = []
    if dup_skus:
        skus = [sku for sku, _ in dup_skus]
        low.append(f"Resolve {len(dup_skus)} duplicate SKU(s): {truncate_list(skus, 5)}")

    return {
        "duplicates_score": score,
        "exact_duplicates": exact_duplicates,
        "fuzzy_duplicates": fuzzy_duplicates,
        "duplicate_skus": duplicate_skus,
        "high_priority_actions": "; ".join(high),
        "medium_priority_actions": "; ".join(medium),
        "low_priority_actions": "; ".join(low),
        **stats,
    }


# ---------------------------------------------------------------------------
# Pricing
# ---------------------------------------------------------------------------

def analyze_pricing(items, categories, stats):
    cat_names = {c["id"]: (c.get("category_data") or {}).get("name", c["id"]) for c in categories}

    zero_price = []
    for item in items:
        d = item.get("item_data", {})
        for v in (d.get("variations") or []):
            vd = v.get("item_variation_data", {})
            if vd.get("pricing_type") != "VARIABLE_PRICING" and not vd.get("price_money"):
                zero_price.append(f"{d.get('name', '?')}($0.00)")
                break

    cat_prices = defaultdict(list)
    for item in items:
        d = item.get("item_data", {})
        for cid in category_ids(d):
            for v in (d.get("variations") or []):
                vd = v.get("item_variation_data", {})
                pm = vd.get("price_money") or {}
                amount = pm.get("amount", 0)
                if vd.get("pricing_type") != "VARIABLE_PRICING" and amount > 0:
                    cat_prices[cid].append(amount)

    cat_bounds = {}
    for cid, prices in cat_prices.items():
        if len(prices) < 4:
            continue
        s = sorted(prices)
        n = len(s)
        q1, q3, median = s[n // 4], s[(n * 3) // 4], s[n // 2]
        iqr = q3 - q1
        cat_bounds[cid] = {"lower": q1 - 1.5 * iqr, "upper": q3 + 1.5 * iqr, "median": median}

    outliers = []
    for item in items:
        d = item.get("item_data", {})
        flagged = False
        for cid in category_ids(d):
            if flagged:
                break
            bounds = cat_bounds.get(cid)
            if not bounds:
                continue
            for v in (d.get("variations") or []):
                vd = v.get("item_variation_data", {})
                pm = vd.get("price_money") or {}
                amount = pm.get("amount", 0)
                if vd.get("pricing_type") != "VARIABLE_PRICING" and amount > 0:
                    if amount > bounds["upper"] or amount < bounds["lower"]:
                        outliers.append(
                            f"{d.get('name', '?')}({format_price(amount)} vs median "
                            f"{format_price(bounds['median'])} in {cat_names.get(cid, cid)})"
                        )
                        flagged = True
                        break

    score = score_from_thresholds(len(zero_price) + len(outliers), 5)

    high = []
    if zero_price:
        high.append(f"Add prices to {len(zero_price)} item(s): {truncate_list(zero_price, 10)}")
    low = []
    if outliers:
        low.append(f"Review {len(outliers)} price outlier(s): {truncate_list(outliers, 5)}")

    return {
        "pricing_score": score,
        "zero_price_items": "; ".join(zero_price),
        "price_outliers": "; ".join(outliers),
        "high_priority_actions": "; ".join(high),
        "medium_priority_actions": "",
        "low_priority_actions": "; ".join(low),
        **stats,
    }


# ---------------------------------------------------------------------------
# Categories
# ---------------------------------------------------------------------------

def analyze_categories(items, categories, stats):
    cat_names = {c["id"]: (c.get("category_data") or {}).get("name", c["id"]) for c in categories}

    cat_item_counts = defaultdict(int)
    uncategorized = []
    for item in items:
        d = item.get("item_data", {})
        cids = category_ids(d)
        if not cids:
            uncategorized.append(d.get("name", "?"))
        for cid in cids:
            cat_item_counts[cid] += 1

    empty = [cat_names.get(c["id"], c["id"]) for c in categories if cat_item_counts[c["id"]] == 0]
    overcrowded = [
        (cat_names.get(c["id"], c["id"]), cat_item_counts[c["id"]])
        for c in categories if cat_item_counts[c["id"]] > 50
    ]
    single_item = [cat_names.get(c["id"], c["id"]) for c in categories if cat_item_counts[c["id"]] == 1]

    issue_count = sum([bool(empty), bool(overcrowded), bool(single_item), bool(uncategorized)])
    score = score_from_thresholds(issue_count, 3)

    high = []
    if uncategorized:
        high.append(f"Assign categories to {len(uncategorized)} uncategorized item(s): {truncate_list(uncategorized, 10)}")
    medium = []
    if overcrowded:
        medium.append(
            f"Split {len(overcrowded)} overcrowded category/categories: "
            + ", ".join(f"{n}({c} items)" for n, c in overcrowded)
        )
    low = []
    if empty:
        low.append(f"Remove {len(empty)} empty category/categories: {truncate_list(empty, 10)}")
    if single_item:
        low.append(f"Consider merging {len(single_item)} single-item category/categories: {truncate_list(single_item, 10)}")

    return {
        "categories_score": score,
        "empty_categories": "; ".join(empty),
        "overcrowded_categories": "; ".join(f"{n}({c})" for n, c in overcrowded),
        "single_item_categories": "; ".join(single_item),
        "high_priority_actions": "; ".join(high),
        "medium_priority_actions": "; ".join(medium),
        "low_priority_actions": "; ".join(low),
        **stats,
    }


# ---------------------------------------------------------------------------
# Overall score
# ---------------------------------------------------------------------------

def compute_overall_score(report):
    c = SCORE_VALUES.get(report.get("completeness_score", ""), 0)
    d = SCORE_VALUES.get(report.get("duplicates_score", ""), 0)
    p = SCORE_VALUES.get(report.get("pricing_score", ""), 0)
    cat = SCORE_VALUES.get(report.get("categories_score", ""), 0)
    return round(c * 0.4 + d * 0.2 + p * 0.2 + cat * 0.2)


def merge_priority_actions(*reports):
    return {
        "high_priority_actions": "; ".join(filter(None, [r.get("high_priority_actions", "") for r in reports])),
        "medium_priority_actions": "; ".join(filter(None, [r.get("medium_priority_actions", "") for r in reports])),
        "low_priority_actions": "; ".join(filter(None, [r.get("low_priority_actions", "") for r in reports])),
    }


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
    items_raw = raw.get("items") or []
    categories_raw = raw.get("categories") or []

    items = [o for o in items_raw if o.get("type") == "ITEM"]
    categories = [o for o in categories_raw if o.get("type") == "CATEGORY"]

    try:
        stats = compute_stats(items)

        if focus == "completeness":
            report = analyze_completeness(items, stats)
        elif focus == "duplicates":
            report = analyze_duplicates(items, stats)
        elif focus == "pricing":
            report = analyze_pricing(items, categories, stats)
        elif focus == "categories":
            report = analyze_categories(items, categories, stats)
        else:  # full
            c = analyze_completeness(items, stats)
            d = analyze_duplicates(items, stats)
            p = analyze_pricing(items, categories, stats)
            cat = analyze_categories(items, categories, stats)
            report = {
                **stats,
                "completeness_score": c["completeness_score"],
                "top_issue_items": c["top_issue_items"],
                "duplicates_score": d["duplicates_score"],
                "exact_duplicates": d["exact_duplicates"],
                "duplicate_skus": d["duplicate_skus"],
                "pricing_score": p["pricing_score"],
                "zero_price_items": p["zero_price_items"],
                "price_outliers": p["price_outliers"],
                "categories_score": cat["categories_score"],
                "empty_categories": cat["empty_categories"],
                "overcrowded_categories": cat["overcrowded_categories"],
                "single_item_categories": cat["single_item_categories"],
                **merge_priority_actions(c, d, p, cat),
            }
            report["overall_score"] = compute_overall_score(report)

        print(json.dumps({"focus_area": focus, "data": report}, indent=2))

    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
