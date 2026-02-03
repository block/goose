---
name: vibes
description: Generate React web apps with Fireproof database. Use when creating new web applications, adding components, or working with local-first databases. Ideal for quick prototypes and single-page apps that need real-time data sync.
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, AskUserQuestion
---

**Display this ASCII art immediately when starting:**

```
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓███████▓▒░░▒▓████████▓▒░░▒▓███████▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░
 ░▒▓█▓▒▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░
 ░▒▓█▓▒▒▓█▓▒░░▒▓█▓▒░▒▓███████▓▒░░▒▓██████▓▒░  ░▒▓██████▓▒░
  ░▒▓█▓▓█▓▒░ ░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░
  ░▒▓█▓▓█▓▒░ ░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░
   ░▒▓██▓▒░  ░▒▓█▓▒░▒▓███████▓▒░░▒▓████████▓▒░▒▓███████▓▒░
```

## Quick Navigation

- [Pre-Flight Check](#pre-flight-check-connect-status) - Validate Connect setup before coding
- [Core Rules](#core-rules) - Essential guidelines for app generation
- [Generation Process](#generation-process) - Design reasoning and code output
- [Assembly Workflow](#assembly-workflow) - Build the final app
- [UI Style & Theming](#ui-style--theming) - OKLCH colors and design patterns
- [Fireproof API](#fireproof-api) - Database operations and hooks
- [AI Features](#ai-features-optional) - Optional AI integration
- [Common Mistakes](#common-mistakes-to-avoid) - Avoid these pitfalls
- [Deployment Options](#deployment-options) - Where to deploy

---

# Vibes DIY App Generator

Generate React web applications using Fireproof for local-first data persistence.

## Pre-Flight Check: Connect Status

**MANDATORY: Complete these steps BEFORE generating any app code.**

**Step 0: Check Connect Status**

Run this command first to validate all required credentials:
```bash
if test -f "./.env" && \
   grep -qE "^VITE_CLERK_PUBLISHABLE_KEY=pk_(test|live)_" ./.env 2>/dev/null && \
   grep -qE "^VITE_API_URL=" ./.env 2>/dev/null && \
   grep -qE "^VITE_CLOUD_URL=" ./.env 2>/dev/null; then
  echo "CONNECT_READY"
else
  echo "CONNECT_NOT_READY"
fi
```

**If output is "CONNECT_NOT_READY"**, Connect setup is required. Inform the user:

> Connect with Clerk authentication is required for Vibes apps. Let me guide you through the setup.

Then proceed with Connect setup below.

### Connect Setup Steps:

**Step 1: Clone Fireproof Repository** (if not already present)

```bash
# Check if repo exists
if [ ! -d "./fireproof" ]; then
  git clone --branch selem/docker-for-all https://github.com/fireproof-storage/fireproof.git ./fireproof
fi
```

**Step 2: Choose Credential Mode**

Use AskUserQuestion:
```
Question: "How would you like to set up credentials?"
Header: "Credentials"
Options:
- Label: "Fresh credentials (Recommended)"
  Description: "Generate all new session tokens and CA keys. Use this for new projects."
- Label: "Import from file"
  Description: "Load credentials from a colleague's exported file. Use for team sharing."
- Label: "Quick local dev"
  Description: "Use preset dev tokens. Fastest setup but not for production."
```

**Step 3: Gather Clerk Keys**

Collect credentials using AskUserQuestion. Users enter values via the "Other" option.

**3a. Publishable Key:**
```
Question: "Enter your Clerk Publishable Key (paste via 'Other')"
Header: "Publishable"
Options:
- Label: "I need to create a Clerk app first"
  Description: "Go to clerk.com → Create application → Configure → API Keys"
```

After receiving via "Other", validate: must start with `pk_test_` or `pk_live_`.
If invalid, inform the user and ask again.

**3b. Secret Key:**
```
Question: "Enter your Clerk Secret Key (paste via 'Other')"
Header: "Secret Key"
Options:
- Label: "Where do I find this?"
  Description: "Clerk Dashboard → Configure → API Keys → Secret keys section"
```

After receiving via "Other", validate: must start with `sk_test_` or `sk_live_`.
If invalid, inform the user and ask again.

**3c. JWT URL (Auto-derived):**

Extract the Clerk domain from the publishable key - no user input needed:
```javascript
// Publishable key format: pk_test_<base64-encoded-domain>
const payload = publishableKey.replace(/^pk_(test|live)_/, '');
const domain = atob(payload).replace('$', '');
const jwtUrl = `https://${domain}/.well-known/jwks.json`;
```

Example: `pk_test_aW50ZXJuYWwtZGluZ28tMjguY2xlcmsuYWNjb3VudHMuZGV2JA`
→ decodes to `internal-dingo-28.clerk.accounts.dev`
→ JWT URL: `https://internal-dingo-28.clerk.accounts.dev/.well-known/jwks.json`

Tell the user what was derived so they can verify it looks correct.

**Step 3b: Configure JWT Template**

Instruct the user to set up a JWT template in Clerk:

> In your Clerk Dashboard, go to **Configure → Sessions → JWT templates**.
> Click **"Add a new template"** and paste this under **Claims**:
>
> ```json
> {
>     "role": "authenticated",
>     "params": {
>         "last": "{{user.last_name}}",
>         "name": "{{user.username}}",
>         "email": "{{user.primary_email_address}}",
>         "first": "{{user.first_name}}",
>         "image_url": "{{user.image_url}}",
>         "external_id": "{{user.external_id}}",
>         "public_meta": "{{user.public_metadata}}",
>         "email_verified": "{{user.email_verified}}"
>     },
>     "userId": "{{user.id}}"
> }
> ```
>
> Save the template. This ensures Fireproof receives the user info it needs for sync.

**Step 4: Run Setup Script**

```bash
# Fresh credentials (default)
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup-connect.js" \
  --clerk-publishable-key "pk_test_..." \
  --clerk-secret-key "sk_test_..." \
  --clerk-jwt-url "https://your-app.clerk.accounts.dev" \
  --mode fresh

# Import from file
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup-connect.js" \
  --clerk-publishable-key "pk_test_..." \
  --clerk-secret-key "sk_test_..." \
  --clerk-jwt-url "https://your-app.clerk.accounts.dev" \
  --mode import --import-file ./team-credentials.txt

# Quick dev (uses preset tokens)
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup-connect.js" \
  --clerk-publishable-key "pk_test_..." \
  --clerk-secret-key "sk_test_..." \
  --clerk-jwt-url "https://your-app.clerk.accounts.dev" \
  --mode quick-dev
```

**Step 5: Show Docker Instructions**

After successful setup, tell the user:
```
Connect setup complete!

To start the Fireproof services:
  cd fireproof/core && docker compose up --build

Services will be available at:
  - Token API: http://localhost:8080/api/
  - Cloud Sync: fpcloud://localhost:8080?protocol=ws

Your .env file has been created. Apps you generate will auto-detect Connect.
```

**If Connect IS set up** (CONNECT_READY), proceed directly to app generation. The assemble script will populate Connect config from .env.

**Platform Name vs User Intent**: "Vibes" is the name of this app platform (Vibes DIY). When users say "vibe" or "vibes" in their prompt, interpret it as:
- Their project/brand name ("my vibes tracker")
- A positive descriptor ("good vibes app")
- NOT as "mood/atmosphere" literally

Do not default to ambient mood generators, floating orbs, or meditation apps unless explicitly requested.

**Import Map Note**: The import map aliases `use-fireproof` to `@necrodome/fireproof-clerk`. Your code uses `import { useFireproofClerk } from "use-fireproof"` and the browser resolves this to the `@necrodome/fireproof-clerk` package, which provides Clerk authentication integration and cloud sync support. This is intentional—it ensures compatible versions and enables auth when Connect is configured.

## Core Rules

- **Use JSX** - Standard React syntax with Babel transpilation
- **Single HTML file** - App code assembled into template
- **Fireproof for data** - Use `useFireproofClerk` for database + sync
- **Auto-detect Connect** - Template handles Clerk auth when Connect is configured
- **Tailwind for styling** - Mobile-first, responsive design

## Generation Process

### Step 1: Design Reasoning

Before writing code, reason about the design in `<design>` tags:

```
<design>
- What is the core functionality and user flow?
- What OKLCH colors fit this theme? (dark/light, warm/cool, vibrant/muted)
- What layout best serves the content? (cards, list, dashboard, single-focus)
- What micro-interactions would feel satisfying? (hover states, transitions)
- What visual style matches the purpose? (minimal, bold, playful, professional)
</design>
```

### Step 2: Output Code

After reasoning, output the complete JSX in `<code>` tags:

```
<code>
import React, { useState } from "react";
import { useFireproofClerk } from "use-fireproof";

export default function App() {
  const { database, useLiveQuery, useDocument, syncStatus } = useFireproofClerk("app-name-db");
  // ... component logic

  return (
    <div className="min-h-screen bg-[#f1f5f9] p-4">
      {/* Sync status indicator (optional) */}
      <div className="text-xs text-gray-500 mb-2">Sync: {syncStatus}</div>
      {/* Your app UI */}
    </div>
  );
}
</code>
```

**⚠️ CRITICAL: Fireproof Hook Pattern**

The `@necrodome/fireproof-clerk` package exports ONLY `useFireproofClerk`. Always use this pattern:

```jsx
// ✅ CORRECT - This is the ONLY pattern that works
import { useFireproofClerk } from "use-fireproof";
const { database, useDocument, useLiveQuery, syncStatus } = useFireproofClerk("my-db");
const { doc, merge } = useDocument({ _id: "doc1" });

// ❌ WRONG - DO NOT USE (old use-vibes API)
import { toCloud, useFireproof } from "use-fireproof";  // WRONG - old API
import { useDocument } from "use-fireproof";  // WRONG - standalone import
const { attach } = useFireproof("db", { attach: toCloud() });  // WRONG - old pattern
```

**Sync Status**: `syncStatus` provides the current sync state as a string. Display it for user feedback.

**Connect Configuration**: Generated apps require Clerk authentication and cloud sync.
The `assemble.js` script populates `window.__VIBES_CONFIG__` from your `.env` file.
Apps will show a configuration error if credentials are missing.

## Assembly Workflow

1. Extract the code from `<code>` tags and write to `app.jsx`
2. Optionally save `<design>` content to `design.md` for documentation
3. Run assembly:
   ```bash
   node "${CLAUDE_PLUGIN_ROOT}/scripts/assemble.js" app.jsx index.html
   ```
4. Tell user: "Open `index.html` in your browser to view your app."

---

> **⚠️ DEPRECATED API:** Never use the old `useFireproof` with `toCloud()` pattern. See [references/DEPRECATED.md](references/DEPRECATED.md) for migration details if you encounter legacy code.

---

## UI Style & Theming

### OKLCH Colors (Recommended)

Use OKLCH for predictable, vibrant colors. Unlike RGB/HSL, OKLCH has perceptual lightness - changing L by 10% looks the same across all hues.

```css
oklch(L C H)
/* L = Lightness (0-1): 0 black, 1 white */
/* C = Chroma (0-0.4): 0 gray, higher = more saturated */
/* H = Hue (0-360): color wheel degrees */
```

**Theme-appropriate palettes:**

```jsx
{/* Dark/moody theme */}
className="bg-[oklch(0.15_0.02_250)]"  /* Deep blue-black */

{/* Warm/cozy theme */}
className="bg-[oklch(0.25_0.08_30)]"   /* Warm brown */

{/* Fresh/bright theme */}
className="bg-[oklch(0.95_0.03_150)]"  /* Mint white */

{/* Vibrant accent */}
className="bg-[oklch(0.7_0.2_145)]"    /* Vivid green */
```

### Better Gradients with OKLCH

Use `in oklch` for smooth gradients without muddy middle zones:

```jsx
{/* Smooth gradient - no gray middle */}
className="bg-[linear-gradient(in_oklch,oklch(0.6_0.2_250),oklch(0.6_0.2_150))]"

{/* Sunset gradient */}
className="bg-[linear-gradient(135deg_in_oklch,oklch(0.7_0.25_30),oklch(0.5_0.2_330))]"

{/* Dark glass effect */}
className="bg-[linear-gradient(180deg_in_oklch,oklch(0.2_0.05_270),oklch(0.1_0.02_250))]"
```

### Neobrute Style (Optional)

For bold, graphic UI:

- **Borders**: thick 4px, dark `border-[#0f172a]`
- **Shadows**: hard offset `shadow-[6px_6px_0px_#0f172a]`
- **Corners**: square (0px) OR pill (rounded-full) - no in-between

```jsx
<button className="px-6 py-3 bg-[oklch(0.95_0.02_90)] border-4 border-[#0f172a] shadow-[6px_6px_0px_#0f172a] hover:shadow-[4px_4px_0px_#0f172a] font-bold">
  Click Me
</button>
```

### Glass Morphism (Dark themes)

```jsx
<div className="bg-white/5 backdrop-blur-lg border border-white/10 rounded-2xl">
  {/* content */}
</div>
```

### Color Modifications

Lighten/darken using L value:
- **Hover**: increase L by 0.05-0.1
- **Active/pressed**: decrease L by 0.05
- **Disabled**: reduce C to near 0

---

## Fireproof API

Fireproof is a local-first database - no loading or error states required, just empty data states. Data persists across sessions and syncs in real-time when Connect is configured.

### Setup
```jsx
import { useFireproofClerk } from "use-fireproof";

const { database, useLiveQuery, useDocument, syncStatus } = useFireproofClerk("my-app-db");
```

**Note**: When Connect is configured (via .env), the template wraps your App in `ClerkFireproofProvider`, enabling authenticated cloud sync automatically. Your code just uses `useFireproofClerk`.

### Choosing Your Pattern

**useDocument** = Form-like editing. Accumulate changes with `merge()`, then save with `submit()` or `save()`. Best for: text inputs, multi-field forms, editing workflows.

**database.put() + useLiveQuery** = Immediate state changes. Each action writes directly. Best for: counters, toggles, buttons, any single-action updates.

```jsx
// FORM PATTERN: User types, then submits
const { doc, merge, submit } = useDocument({ title: "", body: "", type: "post" });
// merge({ title: "..." }) on each keystroke, submit() when done

// IMMEDIATE PATTERN: Each click is a complete action
const { docs } = useLiveQuery("_id", { key: "counter" });
const count = docs[0]?.value || 0;
const increment = () => database.put({ _id: "counter", value: count + 1 });
```

### useDocument - Form State (NOT useState)

**IMPORTANT**: Don't use `useState()` for form data. Use `merge()` and `submit()` from `useDocument`. Only use `useState` for ephemeral UI state (active tabs, open/closed panels).

```jsx
// Create new documents (auto-generated _id recommended)
const { doc, merge, submit, reset } = useDocument({ text: "", type: "item" });

// Edit existing document by known _id
const { doc, merge, save } = useDocument({ _id: "user-profile:abc@example.com" });

// Methods:
// - merge(updates) - update fields: merge({ text: "new value" })
// - submit(e) - save + reset (for forms creating new items)
// - save() - save without reset (for editing existing items)
// - reset() - discard changes
```

### useLiveQuery - Real-time Lists

```jsx
// Simple: query by field value
const { docs } = useLiveQuery("type", { key: "item" });

// Recent items (_id is roughly temporal - great for simple sorting)
const { docs } = useLiveQuery("_id", { descending: true, limit: 100 });

// Range query
const { docs } = useLiveQuery("rating", { range: [3, 5] });
```

**CRITICAL**: Custom index functions are SANDBOXED and CANNOT access external variables. Query all, filter in render:

```jsx
// GOOD: Query all, filter in render
const { docs: allItems } = useLiveQuery("type", { key: "item" });
const filtered = allItems.filter(d => d.category === selectedCategory);
```

### Direct Database Operations
```jsx
// Create/update
const { id } = await database.put({ text: "hello", type: "item" });

// Delete
await database.del(item._id);
```

### Common Pattern - Form + List
```jsx
import React from "react";
import { useFireproofClerk } from "use-fireproof";

export default function App() {
  const { database, useLiveQuery, useDocument, syncStatus } = useFireproofClerk("my-db");

  // Form for new items (submit resets for next entry)
  const { doc, merge, submit } = useDocument({ text: "", type: "item" });

  // Live list of all items of type "item"
  const { docs } = useLiveQuery("type", { key: "item" });

  return (
    <div className="min-h-screen bg-[#f1f5f9] p-4">
      {/* Optional sync status indicator */}
      <div className="text-xs text-gray-500 mb-2">Sync: {syncStatus}</div>
      <form onSubmit={submit} className="mb-4">
        <input
          value={doc.text}
          onChange={(e) => merge({ text: e.target.value })}
          className="w-full px-4 py-3 border-4 border-[#0f172a]"
        />
        <button type="submit" className="mt-2 px-4 py-2 bg-[#0f172a] text-[#f1f5f9]">
          Add
        </button>
      </form>
      {docs.map(item => (
        <div key={item._id} className="p-2 mb-2 bg-white border-4 border-[#0f172a]">
          {item.text}
          <button onClick={() => database.del(item._id)} className="ml-2 text-red-500">
            Delete
          </button>
        </div>
      ))}
    </div>
  );
}
```

---

## AI Features (Optional)

If the user's prompt suggests AI-powered features (chatbot, summarization, content generation, etc.), the app needs AI capabilities via the `useAI` hook.

### Detecting AI Requirements

Look for these patterns in the user's prompt:
- "chatbot", "chat with AI", "ask AI"
- "summarize", "generate", "write", "create content"
- "analyze", "classify", "recommend"
- "AI-powered", "intelligent", "smart" (in context of features)

### Collecting OpenRouter Key

When AI is needed, ask the user:

> This app needs AI capabilities. Please provide your OpenRouter API key.
> Get one at: https://openrouter.ai/keys

Store the key for use with the `--ai-key` flag during deployment.

### Using the useAI Hook

The `useAI` hook is automatically included in the template when AI features are detected:

```jsx
import React from "react";
import { useFireproofClerk } from "use-fireproof";

export default function App() {
  const { database, useLiveQuery, syncStatus } = useFireproofClerk("ai-chat-db");
  const { callAI, loading, error } = useAI();

  const handleSend = async (message) => {
    // Save user message
    await database.put({ role: "user", content: message, type: "message" });

    // Call AI
    const response = await callAI({
      model: "anthropic/claude-sonnet-4",
      messages: [{ role: "user", content: message }]
    });

    // Save AI response
    const aiMessage = response.choices[0].message.content;
    await database.put({ role: "assistant", content: aiMessage, type: "message" });
  };

  // Handle limit exceeded
  if (error?.code === 'LIMIT_EXCEEDED') {
    return (
      <div className="p-4 bg-amber-100 text-amber-800 rounded">
        AI usage limit reached. Please wait for monthly reset or upgrade your plan.
      </div>
    );
  }

  // ... rest of UI
}
```

### useAI API

```jsx
const { callAI, loading, error, clearError } = useAI();

// callAI options
await callAI({
  model: "anthropic/claude-sonnet-4",  // or other OpenRouter models
  messages: [
    { role: "system", content: "You are a helpful assistant." },
    { role: "user", content: "Hello!" }
  ],
  temperature: 0.7,  // optional
  max_tokens: 1000   // optional
});

// error structure
error = {
  code: "LIMIT_EXCEEDED" | "API_ERROR" | "NETWORK_ERROR",
  message: "Human-readable error message"
}
```

### Deployment with AI

When deploying AI-enabled apps, include the OpenRouter key:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" \
  --name myapp \
  --file index.html \
  --ai-key "sk-or-v1-your-key"
```

---

## Common Mistakes to Avoid

- **DON'T** use `useState` for form fields - use `useDocument`
- **DON'T** use `Fireproof.fireproof()` - use `useFireproofClerk()` hook
- **DON'T** use the old `useFireproof` with `toCloud()` - use `useFireproofClerk` instead
- **DON'T** use white text on light backgrounds
- **DON'T** use `call-ai` directly - use `useAI` hook instead (it handles proxying and limits)

---

## Deployment Options

After generating your app, you can deploy it:

- **exe.dev** - VM hosting with nginx. Use `/vibes:exe` to deploy.
- **Local** - Just open `index.html` in your browser. Works offline with Fireproof.

---

## What's Next?

After generating and assembling the app, present these options using AskUserQuestion:

```
Question: "Your app is ready! What would you like to do next?"
Header: "Next"
Options:
- Label: "Keep improving this app"
  Description: "Continue iterating on what you've built. Add new features, refine the styling, or adjust functionality. Great when you have a clear vision and want to polish it further."

- Label: "Apply a design reference (/design-reference)"
  Description: "Have a design.html or mockup file? This skill mechanically transforms your app to match it exactly - pixel-perfect fidelity with your Fireproof data binding preserved."

- Label: "Explore variations (/riff)"
  Description: "Not sure if this is the best approach? Riff generates 3-10 completely different interpretations of your idea in parallel. You'll get ranked variations with business model analysis to help you pick the winner."

- Label: "Make it a SaaS (/sell)"
  Description: "Ready to monetize? Sell transforms your app into a multi-tenant SaaS with Clerk authentication, subscription billing, and isolated databases per customer. Each user gets their own subdomain."

- Label: "Deploy to exe.dev (/exe)"
  Description: "Go live right now. Deploy creates a persistent VM at yourapp.exe.xyz with HTTPS, nginx, and Claude pre-installed for remote development. Your app stays online even when you close your laptop."

- Label: "I'm done for now"
  Description: "Wrap up this session. Your files are saved locally - come back anytime to continue."
```

**After user responds:**
- "Keep improving" → Acknowledge and stay ready for iteration prompts
- "Apply a design reference" → Auto-invoke /vibes:design-reference skill
- "Explore variations" → Auto-invoke /vibes:riff skill
- "Make it a SaaS" → Auto-invoke /vibes:sell skill
- "Deploy" → Auto-invoke /vibes:exe skill
- "I'm done" → Confirm files saved, wish them well

**Do NOT proceed to code generation until:**
Connect setup is complete with valid Clerk credentials in .env (pre-flight check returns CONNECT_READY).
