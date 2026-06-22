## New extension submission

<!--
Use this template when adding an MCP server to the goose extension catalog
(documentation/static/servers.json + a docs/mcp/ tutorial page).

Full criteria: https://goose-docs.ai/docs/mcp/contributing
Narrow/personal servers don't need to be in the catalog - they can be added
locally as a custom extension instead.
-->

**Extension name:**
**Source / repo:**
**Maintainer (you / the vendor / community):**

### Summary
<!-- What does this extension do and who is it useful for? -->

### Provenance & trust
- [ ] Source is open and inspectable (or closed/remote status is clearly noted)
- [ ] Official vendor integration, or clearly marked as a community reimplementation
- [ ] Repo is actively maintained (recent commits, real releases)

### Security
- [ ] Uses OAuth / scoped tokens where possible (not long-lived keys)
- [ ] Secrets come from env vars / keychain, **never** inlined in `goose://` links or examples
- [ ] Requests least-privilege scopes
- [ ] Destructive or money-spending tools are documented
- [ ] Data egress / privacy is documented (for remote servers)

### Quality
- [ ] Tutorial page based on `_template_.mdx`
- [ ] Install verified on **goose Desktop**
- [ ] Install verified on **goose CLI**
- [ ] Tool surface is focused (not dozens of unrelated tools)

### Transparency (if commercial)
- [ ] Pricing is disclosed (free / paid / freemium)
- [ ] A sandbox or test mode exists, or limitations are stated
- [ ] License / commercial terms are clear

### Catalog entry
- [ ] `servers.json` entry added with required fields
- [ ] `npm run validate:servers` passes locally
- [ ] Automated security scan reviewed (will run on this PR)

### Related issues / discussion
Relates to #ISSUE_ID
