#!/usr/bin/env python3
"""Validate email campaign payloads — stdlib only (no pydantic).

Usage:
    echo '{"email_campaign":{...}}' | python3 validate-email-campaign.py
    python3 validate-email-campaign.py '{"email_campaign":{...}}'
    python3 validate-email-campaign.py --print-schema
"""

import argparse
import json
import sys

# -- Schema definition (mirrors pydantic version) --

BLOCK_TYPES = {
    "settings", "header", "text", "item", "button", "image",
    "spacer", "divider", "coupon_campaign", "footer",
}

TEXT_FORMATS = {"h1", "h2", "p"}
TEXT_ALIGNMENTS = {"left", "center", "right"}
BUTTON_ACTIONS = {"link"}
BUTTON_SHAPES = {"curved", "square"}
BUTTON_FILLS = {"solid", "outline"}
BUTTON_WIDTHS = {"full", "auto"}
BUTTON_SIZES = {"small", "medium", "large"}
CHANNELS = {"email", "sms", "facebook"}
CAMPAIGN_TYPES = {"announcement", "coupon", "event"}

WRONG_BODY_FIELDS = {"body_content", "sections", "widgets"}

SCHEMA = {
    "email_campaign": {
        "name": {"type": "string", "required": True},
        "subject": {"type": "string", "required": True},
        "channel": {"type": "string", "values": sorted(CHANNELS), "default": "email"},
        "campaign_type": {"type": "string", "values": sorted(CAMPAIGN_TYPES), "default": "announcement"},
        "social_enabled": {"type": "boolean", "default": True},
        "theme_campaign_id": {"type": "integer", "required": True},
        "body": {
            "type": "array",
            "required": True,
            "description": "Flat array of content blocks. First must be header, last must be footer.",
            "block_types": {
                "header": {"fields": {"id": "string (required)", "profileImageUrl": "string", "businessName": "string", "showLogo": "boolean", "showMerchantName": "boolean", "hideBusinessInfo": "boolean"}},
                "footer": {"fields": {"id": "string (required)", "socialLinks": "array", "unsubscribeText": "string", "facebookUrl": "string", "instagramUsername": "string", "twitterUsername": "string", "websiteUrl": "string"}},
                "text": {"fields": {"id": "string (required)", "text": "string (required)", "format": "h1|h2|p (required)", "alignment": "left|center|right"}},
                "item": {"fields": {"id": "string (required)", "name": "string (required)", "price": "string (required)", "image": "string", "linkUrl": "string"}},
                "button": {"fields": {"id": "string (required)", "action": "link (required)", "label": "string (required)", "link": "string (required)", "shape": "curved|square", "fill": "solid|outline", "width": "full|auto", "size": "small|medium|large"}},
                "image": {"fields": {"id": "string (required)", "src": "string (required)", "alt": "string"}},
                "spacer": {"fields": {"id": "string (required)", "height": "integer"}},
                "divider": {"fields": {"id": "string (required)"}},
                "settings": {"fields": {"id": "string (required)", "contentWrapPaddingDisabled": "boolean"}},
                "coupon_campaign": {"fields": {"id": "string (required)", "coupon_campaign_id": "string"}},
            },
        },
        "style": {"type": "object", "fields": {"cardColor": "string"}},
        "source": {"type": "string"},
        "coupon_campaign_v2": {"type": "object", "fields": {"pricing_rule_id": "string", "pricing_rule_version": "string", "expires_in_days": "integer"}},
    },
}


def validate(data: dict) -> list[str]:
    """Return list of error strings. Empty = valid."""
    errors = []

    if not isinstance(data, dict):
        return ["Payload must be a JSON object"]

    if "email_campaign" not in data:
        return ["Missing top-level 'email_campaign' object"]

    ec = data["email_campaign"]
    if not isinstance(ec, dict):
        return ["'email_campaign' must be a JSON object"]

    # Reject wrong body field names
    for bad in WRONG_BODY_FIELDS:
        if bad in ec:
            errors.append(f"Found '{bad}' — the API expects 'body' (flat array), not '{bad}'. Remove it.")

    # Required fields
    if not ec.get("name"):
        errors.append("email_campaign.name is required")
    if not ec.get("subject"):
        errors.append("email_campaign.subject is required")
    if "theme_campaign_id" not in ec:
        errors.append("email_campaign.theme_campaign_id is required")
    elif not isinstance(ec["theme_campaign_id"], int):
        errors.append("email_campaign.theme_campaign_id must be an integer")

    # Channel
    if "channel" in ec and ec["channel"] not in CHANNELS:
        errors.append(f"email_campaign.channel must be one of {sorted(CHANNELS)}")

    # Campaign type
    if "campaign_type" in ec and ec["campaign_type"] not in CAMPAIGN_TYPES:
        errors.append(f"email_campaign.campaign_type must be one of {sorted(CAMPAIGN_TYPES)}")

    # Body
    body = ec.get("body")
    if body is None:
        errors.append("email_campaign.body is required")
        return errors

    if not isinstance(body, list):
        errors.append("email_campaign.body must be an array")
        return errors

    if len(body) == 0:
        errors.append("email_campaign.body must have at least one block")
        return errors

    # First/last block
    if body[0].get("type") != "header":
        errors.append("First body block must be type 'header'")
    if body[-1].get("type") != "footer":
        errors.append("Last body block must be type 'footer'")

    # Block validation
    seen_ids = set()
    for i, block in enumerate(body):
        if not isinstance(block, dict):
            errors.append(f"body[{i}]: block must be a JSON object")
            continue

        btype = block.get("type")
        if not btype:
            errors.append(f"body[{i}]: missing 'type'")
            continue
        if btype not in BLOCK_TYPES:
            errors.append(f"body[{i}]: unknown type '{btype}' (valid: {sorted(BLOCK_TYPES)})")
            continue

        bid = block.get("id")
        if not bid:
            errors.append(f"body[{i}] ({btype}): missing 'id'")
        elif bid in seen_ids:
            errors.append(f"body[{i}] ({btype}): duplicate id '{bid}'")
        else:
            seen_ids.add(bid)

        # Type-specific required fields
        if btype == "text":
            if not block.get("text"):
                errors.append(f"body[{i}] (text): 'text' is required")
            fmt = block.get("format")
            if not fmt:
                errors.append(f"body[{i}] (text): 'format' is required (h1, h2, or p)")
            elif fmt not in TEXT_FORMATS:
                errors.append(f"body[{i}] (text): format must be one of {sorted(TEXT_FORMATS)}")
            alignment = block.get("alignment")
            if alignment and alignment not in TEXT_ALIGNMENTS:
                errors.append(f"body[{i}] (text): alignment must be one of {sorted(TEXT_ALIGNMENTS)}")

        elif btype == "item":
            if not block.get("name"):
                errors.append(f"body[{i}] (item): 'name' is required")
            if not block.get("price"):
                errors.append(f"body[{i}] (item): 'price' is required (e.g. '$5.50')")

        elif btype == "button":
            if block.get("action") != "link":
                errors.append(f"body[{i}] (button): 'action' must be 'link'")
            if not block.get("label"):
                errors.append(f"body[{i}] (button): 'label' is required")
            if not block.get("link"):
                errors.append(f"body[{i}] (button): 'link' is required")

        elif btype == "image":
            if not block.get("src"):
                errors.append(f"body[{i}] (image): 'src' is required")

    return errors


def print_schema():
    print(json.dumps(SCHEMA, indent=2))


def main():
    parser = argparse.ArgumentParser(description="Validate email campaign JSON payloads.")
    parser.add_argument("payload", nargs="?", help="JSON string. Reads from stdin if omitted.")
    parser.add_argument("--print-schema", action="store_true", help="Print schema and exit.")
    args = parser.parse_args()

    if args.print_schema:
        print_schema()
        return

    raw = args.payload if args.payload is not None else sys.stdin.read()

    try:
        data = json.loads(raw)
    except json.JSONDecodeError as e:
        print(f"FAIL: invalid JSON — {e}", file=sys.stderr)
        sys.exit(1)

    errors = validate(data)
    if errors:
        print(f"FAIL: {len(errors)} error(s) found:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        sys.exit(1)
    else:
        print("OK: email campaign payload is valid")


if __name__ == "__main__":
    main()
