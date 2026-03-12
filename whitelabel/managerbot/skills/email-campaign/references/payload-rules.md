# Email Campaign Payload Rules

Read this when drafting or debugging the campaign JSON payload.

For larger payloads, write the JSON to a temp file first instead of inlining it in the shell. That avoids escaping quotes and makes revisions easier.

## Structural rules

- Use `email_campaign.body`. Do not send `body_content`, `sections`, or `widgets`.
- `body` must be a flat array of blocks.
- The first body block must be `header`.
- The last body block must be `footer`.
- Every block must have a unique `id`.

## Context-dependent rules

- `theme_campaign_id` is required. Get it from `square email-campaigns themes list`.
- If a featured catalog item has an image URL, include it in the `item.image` field.
- For CTA buttons, use the first published site from `square sites list` and construct the URL as `https://<domain>`.
- If no published site exists, omit the button unless the merchant explicitly wants a placeholder link.

## Minimal shape

```json
{
  "email_campaign": {
    "name": "Spring Menu Launch",
    "subject": "Try our new spring menu",
    "theme_campaign_id": 1317,
    "body": [
      { "type": "header", "id": "header-1" },
      { "type": "text", "id": "text-2", "text": "New this week", "format": "h1" },
      { "type": "footer", "id": "footer-3" }
    ]
  }
}
```
