---
name: Email Campaign Creation
description: >
  Create Square Email Marketing campaigns. Use when the merchant wants to promote
  products, announce events, send a newsletter, draft marketing emails, run a sale,
  do seasonal promotions, or anything related to email outreach — even if they don't
  explicitly say "email campaign."
---

# Email Campaign Creation

Create email marketing campaigns for announcements and promotions.

> **Script paths:** Replace `$SKILL_DIR` below with the absolute skill directory path shown in the "Supporting Files" or "Skill directory:" section above.

## Step 1: Gather campaign context

Run the data-gathering script to collect catalog items, recent campaigns, themes, and brand style in one call:

```bash
$SKILL_DIR/scripts/gather-campaign-context.sh --itemKeywords "<keyword1,keyword2>"
```

Omit `--itemKeywords` if the campaign isn't tied to specific products — the script fetches all catalog items so you have context on what to feature.

This orchestrates catalog search, recent campaigns, themes, user style, and sites — all with graceful fallbacks. The output is consolidated JSON.

If the request is vague (no specific product, event, or offer mentioned), ask the merchant what to promote before gathering context.

## Step 2: Draft campaign content

Check the schema first — it's the source of truth for field names and allowed values:

```bash
python3 $SKILL_DIR/scripts/validate-email-campaign.py --print-schema
```

Then read [references/payload-rules.md](references/payload-rules.md) for the non-obvious structural rules.

Using gathered context, build the full `email_campaign` JSON payload:
- Match tone to the merchant's recent campaigns
- Lead with the key announcement, include concrete product details from catalog data
- If an item has an `image_url` in the gathered context, include it in the item block
- For button links, use the first published site from `sites.sites[]`

Write the payload to a temp file (avoids shell escaping issues):

```bash
cat > /tmp/email-campaign.json << 'EOF'
{
  "email_campaign": {
    "name": "Spring Menu Launch",
    "subject": "Try our new spring menu",
    "theme_campaign_id": 1317,
    "body": [
      { "type": "header", "id": "header-1" },
      { "type": "text", "id": "text-1", "text": "New this week", "format": "h1" },
      { "type": "text", "id": "text-2", "text": "Come try our latest additions.", "format": "p" },
      { "type": "item", "id": "item-1", "name": "Matcha Latte", "price": "$5.50" },
      { "type": "button", "id": "btn-1", "action": "link", "label": "Order Now", "link": "https://mystore.square.site" },
      { "type": "footer", "id": "footer-1" }
    ]
  }
}
EOF
```

## Step 3: Preview for merchant approval

Before creating via the API, describe the campaign to the merchant:
- Campaign name and subject line
- Body content in plain language (which blocks, which items featured)
- Ask "Want me to go ahead?" — do not create without confirmation

## Step 4: Validate and create

Validate the payload first:

```bash
python3 $SKILL_DIR/scripts/validate-email-campaign.py < /tmp/email-campaign.json
```

If it prints `OK`, proceed. If errors, fix them before calling create.

Create the campaign:

```bash
square email-campaigns create --body "$(cat /tmp/email-campaign.json)"
```

Campaigns are created as **drafts** — the merchant must still schedule or send from their Square dashboard.

## Payload rules summary

- `body` is a flat array of blocks (NOT `body_content`, `sections`, or `widgets`)
- First block must be `header`, last must be `footer`
- Every block needs a unique `id`
- Block types: header, footer, text, item, button, image, spacer, divider
- Text blocks: `format` is "h1", "h2", or "p"
- Item blocks: `name`, `price` (formatted like "$5.50"), optional `image` URL
- Button blocks: `action: "link"`, `label`, `link` URL
- `theme_campaign_id` is required — get it from the gather script output

## Tips

- Match tone to the merchant's existing campaigns
- If a catalog item has an image, include it — campaigns with images perform better
- Keep subject lines short and direct
- Write payload to `/tmp/email-campaign.json` not inline — avoids `$` shell expansion issues
