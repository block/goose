---
name: Email Campaign
description: >
  Create Square Email Marketing campaigns. Use when the merchant wants to promote
  products, announce events, send a newsletter, draft marketing emails, run a sale,
  or anything related to email outreach.
---

# Email Campaign Creation

## Step 1: Gather context

Run `square email-campaigns help` first to understand the API.

Then gather what you need:

```bash
square email-campaigns themes list --channel email --themes classic
square email-campaigns user-style get
square sites list
```

If the campaign features specific products, search for them:
```bash
square catalog search --body '{"object_types":["ITEM"],"query":{"text_query":{"keywords":["matcha"]}}}'
```

Note the `theme_campaign_id` from themes — you need it for the payload.
For button links, use the first published site from `sites list`.

## Step 2: Draft the campaign

Build a JSON payload. Minimal shape:

```json
{
  "email_campaign": {
    "name": "Spring Menu Launch",
    "subject": "Try our new spring menu",
    "theme_campaign_id": 1317,
    "body": [
      { "type": "header", "id": "header-1" },
      { "type": "text", "id": "text-1", "text": "New this week", "format": "h1" },
      { "type": "text", "id": "text-2", "text": "Come try our latest additions.", "format": "p" },
      { "type": "footer", "id": "footer-1" }
    ]
  }
}
```

Rules:
- `body` is a flat array of blocks
- First block must be `header`, last must be `footer`
- Every block needs a unique `id`
- Block types: header, footer, text, item, button, image, spacer, divider
- Text blocks: `format` is "h1", "h2", or "p"
- Item blocks: `name`, `price` (formatted like "$5.50"), optional `image` URL
- Button blocks: `action: "link"`, `label`, `link` URL

For larger payloads, write to a temp file:
```bash
cat > /tmp/campaign.json << 'EOF'
{ ... }
EOF
square email-campaigns create --body "$(cat /tmp/campaign.json)"
```

## Step 3: Preview and create

Describe the campaign to the merchant before creating. Once approved:

```bash
square email-campaigns create --body '...'
```

Campaigns are created as drafts — the merchant must schedule/send from their dashboard.
In agent mode, this is staged as a mutation for merchant approval.

## Tips

- Match tone to the merchant's existing campaigns if any come back from `list`.
- If a catalog item has an image, include it in the item block.
- Keep subject lines short and direct.
