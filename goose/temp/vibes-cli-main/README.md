# Vibes CLI - a Claude Code skill for vibe coding small apps

![Vibes](assets/vibes.png)

Instantly make your own small multi-user apps, without a backend. With Vibes, The front-end _is_ the app.

## What is Vibes CLI?

Vibes is the vibe coding stack made for AI agents. It collapses application code and application state into a single HTML file that runs anywhere.

**Why does this matter?** AI doesn't make apps - it makes *text*. By embedding the database in JavaScript (via [Fireproof](https://fireproof.storage)), your coding agent can describe an entire app - including its persistence layer - in one shot. No server setup, no schema imports. Just a working app.

Your data lives locally in the browser, encrypted and portable. It syncs across users automatically. Share your creations with a simple link and friends can jump in immediately.

## Quick Start

1. Install the plugin:
   ```
   /plugin marketplace add popmechanic/vibes-cli
   /plugin install vibes@vibes-cli
   ```

2. Restart Claude Code

3. Try it:
   ```
   Make me a todo vibe
   ```

That's it. Claude generates a single HTML file with a working app.

## Installation

In Claude Code, run:

```
/plugin marketplace add popmechanic/vibes-cli
/plugin install vibes@vibes-cli
```

**Important**: Restart Claude Code after installation to load the new skills.

### Updating

To update to the latest version:

```
/plugin update vibes@vibes-cli
```

### Troubleshooting

If updates aren't working or you're stuck on an old version:

```
/plugin marketplace remove vibes-cli
/plugin uninstall vibes@vibes-cli
/plugin marketplace add popmechanic/vibes-cli
/plugin install vibes@vibes-cli
```

Then restart Claude Code.

## Skills

Skills are **model-invoked** - Claude automatically uses them when your task matches the skill's purpose. Just describe what you want to build.

### `vibes`

Generate a complete, working app from a prompt. Perfect when you have a clear idea and want to see it working quickly.

Creates a single HTML file with inline JavaScript, Fireproof database for local-first persistence, and Tailwind CSS styling. No build step - just open and run.

**Example prompts:**
- "Make a chore chart for my roommates"
- "Build a potluck sign-up for Friendsgiving"
- "Create a trivia game about reality TV"

### `design-reference`

Have a design mockup or static HTML file? This skill mechanically transforms it into a working Vibes app while preserving the design exactly.

The transformation is deterministic: CSS is copied verbatim, HTML structure is preserved, only syntax changes (class→className) and dynamic content bindings are added. No interpretation, no "improvements" - pixel-perfect fidelity to your design.

**Example prompts:**
- "Use design.html as the reference for my app"
- "Match this mockup exactly"
- "Convert this static HTML to a Vibes app"

### `riff`

Not sure what to build? Riff generates 3-10 completely different interpretations of your idea in parallel.

Each variation is a genuinely different concept - not just styling changes. You'll get ranked variations with business model analysis to help you pick the winner. Great for exploring a broad idea before committing.

**Example prompt:** "Make me an app that could make money"

**Output:**
```
./
├── index.html          # Gallery showcasing all variations
├── RANKINGS.md         # Scored rankings with recommendations
├── riff-1/
│   ├── index.html      # App variation 1
│   └── BUSINESS.md     # Business model canvas
├── riff-2/
│   └── ...
```

### `sell`

Ready to monetize? Sell transforms your app into a multi-tenant SaaS with Clerk authentication, subscription billing, and isolated databases per customer.

Each user gets their own subdomain (alice.yourapp.com) with their own data. Includes a marketing landing page, admin dashboard, and subscription gating - everything you need to start charging.

![Sell: Multi-Tenant SaaS Philosophy](assets/sell-philosophy.png)

**The philosophy:** Most SaaS turns every community into rows in one database. Sell turns every community into its own world. This is horizontal scaling where the unit is the tenant database—not the server fleet.

**Output:** A single unified `index.html` that handles all routes:
```
yourdomain.com          → Landing page with pricing
*.yourdomain.com        → Tenant app with auth gate
admin.yourdomain.com    → Admin dashboard
```

**Example flow:**
1. Build an app with `/vibes`
2. Run `/sell` to transform it
3. Configure Clerk keys and pricing
4. Deploy with `/exe`

### `exe`

Go live right now. Deploy creates a persistent VM at yourapp.exe.xyz with HTTPS, nginx, and Claude pre-installed.

Your app stays online 24/7 even when you close your laptop. Zero downtime redeployments let you iterate live. Great for demos, pilots, or production.

## Commands

Commands are **user-invoked** - run them explicitly when needed.

### `sync`

Update the cached documentation and import maps from upstream Vibes DIY sources.

### `update`

Deterministically update an existing Vibes app's infrastructure (import maps, library versions, components) without regenerating your code.

```bash
# Analyze an app (dry-run)
node scripts/update.js path/to/app.html

# Apply updates
node scripts/update.js path/to/app.html --apply

# Batch update a directory
node scripts/update.js ./apps/
```

Useful when you have production apps that need library updates but you don't want to regenerate from scratch.

## Why Vibes?

Every vibe-coded project starts in the vibe zone - the AI understands you, progress is fast, each change moves the app forward.

Then something small goes wrong. A fix that mostly works. An edge case layered on top. You correct it, then correct the correction, and suddenly progress slows to a crawl.

You've drifted out of the vibe zone.

![Vibe Zone](assets/vibezone.png)

**Vibes DIY keeps things simple enough that you stay in the vibe zone.** Single-file apps. Local-first data. No server complexity. The AI can see and understand everything it needs to help you.

## How Data Works

Vibes apps use [Fireproof](https://fireproof.storage), a local-first database:

- **Offline-first**: Apps work without internet, sync when connected
- **Encrypted**: Data is encrypted before leaving the browser
- **Shareable**: Real-time sync across users via cloud relay
- **Portable**: Export your data anytime

The hidden settings menu (gear icon) lets you configure sync for collaboration.

## Client-Side Multi-Tenancy

Traditional SaaS multi-tenancy requires backend code, database configuration, tenant isolation logic, and DevOps expertise. Setup takes weeks.

Vibes eliminates these categories of work entirely.

Each subdomain creates a separate database namespace. Tenant isolation happens automatically—tenant A cannot query tenant B's data because the databases are physically separate. Data leaks become architecturally impossible.

The implementation:

```javascript
const subdomain = window.location.hostname.split('.')[0];
const { database } = useFireproofClerk(`app-${subdomain}`);
```

Three lines. No backend. No database configuration. No tenant middleware.

### What This Enables

- **Indie hackers**: Ship commercial apps in hours, not weeks
- **Designers who code**: Build SaaS without learning DevOps
- **Domain experts**: Package expertise as subscription software
- **Rapid validation**: Deploy real infrastructure for customer pilots

The architecture works best for per-user tools, white-label dashboards, customer portals, and micro-SaaS with independent tenants. Each tenant gets their own world.

### The Constraint Is the Feature

You cannot run global queries across all tenants. Some ideas won't fit. This is groupware—tools for communities, not platforms that own them. No one can be Zuckerberg over a Vibes app. That's the point.

## Links

- [vibes.diy](https://vibes.diy) - Try the web builder
- [Discord](https://discord.gg/vnpWycj4Ta) - Join the community
- [GitHub](https://github.com/VibesDIY) - Open source
- [Substack](https://vibesdiy.substack.com/) - Updates and tutorials
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guide

## License

MIT
