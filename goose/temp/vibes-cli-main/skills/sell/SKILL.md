---
name: sell
description: Transform a Vibes app into a multi-tenant SaaS with subdomain-based tenancy. Adds Clerk authentication, subscription gating, and generates a unified app with landing page, tenant routing, and admin dashboard.
allowed-tools: Read, Write, Bash, Glob, AskUserQuestion
---

**Display this ASCII art immediately when starting:**

```
░▒▓███████▓▒░▒▓████████▓▒░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
 ░▒▓██████▓▒░░▒▓██████▓▒░ ░▒▓█▓▒░      ░▒▓█▓▒░
       ░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
       ░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓███████▓▒░░▒▓████████▓▒░▒▓████████▓▒░▒▓████████▓▒░
```

> **Before starting:** Complete [Clerk Setup](CLERK-SETUP.md) to configure authentication.

## Quick Navigation

- [Critical Rules](#-critical-rules---read-first-) - Read this first
- [Workflow Overview](#workflow-overview) - Step-by-step process
- [Step 1: Detect Existing App](#step-1-detect-existing-app) - Find app to transform
- [Step 2: Gather Configuration](#step-2-gather-all-configuration-upfront) - Collect all settings
- [Step 3: Assemble](#step-3-assemble-do-not-generate-code) - Build the unified app
- [Step 4: Deploy](#step-4-deploy) - Go live with exe.dev
- [Step 5: Clerk Setup](#step-5-clerk-setup-required-before-testing) - Configure authentication
- [Key Components](#key-components) - Routing, TenantContext, SubscriptionGate
- [Testing](#testing) - Local and staging verification
- [Troubleshooting](#troubleshooting) - Common issues and fixes

---

## ⛔ CRITICAL RULES - READ FIRST ⛔

**DO NOT generate code manually.** This skill uses pre-built scripts:

| Step | Script | What it does |
|------|--------|--------------|
| Assembly | `assemble-sell.js` | Generates unified index.html |

**Script location:**
```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/assemble-sell.js" ...
```

**NEVER do these manually:**
- ❌ Write HTML/JSX for landing page, tenant app, or admin dashboard
- ❌ Generate routing logic or authentication code

**ALWAYS do these:**
- ✅ Run `assemble-sell.js` to generate the unified app
- ✅ Use `/vibes:exe` to deploy after assembly

---

# Sell - Transform Vibes to SaaS

This skill uses `assemble-sell.js` to inject the user's app into a pre-built template. The template contains security checks, proper Clerk integration, and Fireproof patterns.

Convert your Vibes app into a multi-tenant SaaS product with:
- Subdomain-based tenancy (alice.yourdomain.com)
- Clerk authentication with passkeys
- Subscription gating via Clerk Billing
- Per-tenant Fireproof database isolation
- Marketing landing page
- Admin dashboard

## Architecture

The sell skill generates a **single index.html** file that handles all routes via client-side subdomain detection:

```
yourdomain.com          → Landing page
*.yourdomain.com        → Tenant app with auth
admin.yourdomain.com    → Admin dashboard
```

This approach simplifies deployment - you upload one file and it handles everything.

---

## Workflow Overview

1. **Detect** existing app (app.jsx or riff selection)
2. **Configure** domain, pricing, and Clerk keys
3. **Assemble** unified app (run assembly script)
4. **Deploy** with `/vibes:exe`

---

## Step 1: Detect Existing App

Look for an existing Vibes app to transform:

```bash
# Check current directory
ls -la app.jsx index.html 2>/dev/null

# Check for riff directories
ls -d riff-* 2>/dev/null
```

**Decision tree:**
- Found `app.jsx` → Use directly
- Found multiple `riff-*/app.jsx` → Ask user to select one
- Found nothing → Tell user to run `/vibes:vibes` first

If riffs exist, ask:
> "I found multiple riff variations. Which one would you like to transform into a SaaS product?"

---

## Step 2: Gather ALL Configuration Upfront

**Use AskUserQuestion to collect all config in 2 batches before proceeding.**

Do NOT ask questions one-by-one. Gather everything upfront, then proceed directly to assembly.

### Batch 1: Core Identity

Use the AskUserQuestion tool with these 5 questions:

```
Question 1: "What should we call this app? (used for database naming, e.g., 'wedding-photos')"
Header: "App Name"
Options: Provide 2 suggestions based on context + user enters via "Other"

Question 2: "What domain will this deploy to?"
Header: "Domain"
Options: ["Use exe.xyz subdomain", "Custom domain"]

Question 3: "Do you want to require paid subscriptions?"
Header: "Billing"
Options: ["No - free access for all", "Yes - subscription required"]

Question 4: "What's your Clerk Publishable Key? (from Clerk Dashboard → API Keys)"
Header: "Clerk Key"
Options: User enters via "Other" (starts with pk_test_ or pk_live_)

Question 5: "What's your Clerk JWKS Public Key? (from Clerk Dashboard → API Keys → 'Show JWT Public Key')"
Header: "JWKS Key"
Options: User enters via "Other" (starts with -----BEGIN PUBLIC KEY-----)
```

### Batch 2: Customization

Use the AskUserQuestion tool with these 3 questions:

```
Question 1: "Display title for your app? (shown in headers and landing page)"
Header: "Title"
Options: Suggest based on app name + user enters via "Other"

Question 2: "Tagline for the landing page?"
Header: "Tagline"
Options: Generate 2 suggestions based on app context + user enters via "Other"

Question 3: "What features should we highlight on the landing page? (comma-separated)"
Header: "Features"
Options: User enters via "Other"
```

### After Receiving Answers

1. If user selected "Custom domain", ask for the domain name
2. Optionally ask for Clerk Webhook Secret (for subscription sync with registry)
3. Admin User IDs default to empty (configured after first deploy - see "What's Next?" section)
4. **Proceed immediately to Step 3 (Assembly)** - no more questions

**IMPORTANT: Clerk has TWO different keys:**
| Key | Format | Purpose |
|-----|--------|---------|
| Publishable Key | `pk_test_...` | Frontend auth (asked in Batch 1) |
| JWKS Public Key | `-----BEGIN PUBLIC KEY-----` | Backend JWT verification (asked in Batch 1) |

The JWKS key is found in Clerk Dashboard → API Keys → scroll down to "Show JWT Public Key".

### Config Values Reference

| Config | Script Flag | Example |
|--------|-------------|---------|
| App Name | `--app-name` | `wedding-photos` |
| Domain | `--domain` | `myapp.exe.xyz` |
| Billing | `--billing-mode` | `off` or `required` |
| Clerk Publishable Key | `--clerk-key` | `pk_test_xxx` |
| Title | `--app-title` | `Wedding Photos` |
| Tagline | `--tagline` | `Share your special day` |
| Features | `--features` | `'["Feature 1","Feature 2"]'` |
| Admin IDs | `--admin-ids` | `'["user_xxx"]'` (default: `'[]'`) |

**Stored for deployment (not used by assemble-sell.js):**
| Config | exe.js Flag | Purpose |
|--------|-------------|---------|
| Clerk JWKS Public Key | `--clerk-key` | Registry JWT verification |
| Clerk Webhook Secret | `--clerk-webhook-secret` | Subscription sync |

---

## Step 3: Assemble (DO NOT GENERATE CODE)

**CRITICAL**: You MUST use the assembly script. Do NOT generate your own HTML/JSX code. The template has been carefully designed with proper security and Clerk integration that will break if you generate code manually.

### 3.1 What app.jsx Should Contain

The app.jsx should contain ONLY the user's App component - not SaaS infrastructure. The template provides:
- CONFIG, CLERK_PUBLISHABLE_KEY, APP_NAME, etc.
- ClerkProvider, TenantProvider, SubscriptionGate
- Landing page, admin dashboard, routing
- **Pricing UI via Clerk's `<PricingTable />`** - DO NOT generate pricing components

**⛔ DO NOT generate in app.jsx:**
- Pricing tiers, plans, or subscription UI
- Landing page sections (hero, features, etc.)
- Authentication UI (sign in, sign up buttons)

The template handles all of this. Pricing is configured in Clerk Dashboard → Billing → Plans, and displayed via Clerk's PricingTable component.

**The assembly script automatically strips:**
- Import statements (template imports everything)
- `export default` (template renders App directly)
- `CONFIG` declarations (template provides its own)
- Template constant declarations

### 3.2 Update App for Tenant Context

The user's app needs to use `useTenant()` for database scoping. Check if their app has a hardcoded database name:

```jsx
// BEFORE: Hardcoded name
const { useLiveQuery } = useFireproofClerk("my-app");

// AFTER: Tenant-aware
const { dbName } = useTenant();
const { useLiveQuery } = useFireproofClerk(dbName);
```

If the app uses a hardcoded name, update it to use `useTenant()`:

1. Find the `useFireproofClerk("...")` call
2. Add `const { dbName } = useTenant();` before it
3. Change to `useFireproofClerk(dbName)`

The template makes `useTenant` available globally via `window.useTenant`.

### 3.3 Assemble Unified App

Run the assembly script to generate the unified file:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/assemble-sell.js" app.jsx index.html \
  --clerk-key "pk_test_xxx" \
  --app-name "wedding-photos" \
  --app-title "Fantasy Wedding" \
  --domain "myapp.exe.xyz" \
  --tagline "Share your wedding photos with guests" \
  --billing-mode "required" \
  --features '["Photo sharing","Guest uploads","Live gallery"]' \
  --admin-ids '["user_xxx"]'
```

**Billing Modes:**
- `off` (default): Everyone gets free access after signing in
- `required`: Users must subscribe via Clerk Billing to access the app

**The assembly script generates:**
- `index.html` - Unified app (landing + tenant + admin)

**WARNING**: If the assembly script fails or isn't available, DO NOT attempt to write the HTML manually. The template is complex and contains critical security patterns. Ask the user to ensure the plugin is installed correctly.

### 3.4 Customize Landing Page Theme (Optional)

The template uses neutral colors by default. To match the user's brand or prompt style, customize the CSS variables in the generated `index.html`:

```css
:root {
  /* Landing page theming - customize these for brand */
  --landing-accent: #0f172a;        /* Primary button/text color */
  --landing-accent-hover: #1e293b;  /* Hover state */
}
```

**Examples based on prompt style:**
- Wedding app → `--landing-accent: #d4a574;` (warm gold)
- Tech startup → `--landing-accent: #6366f1;` (vibrant indigo)
- Health/wellness → `--landing-accent: #10b981;` (fresh green)
- Creative agency → `--landing-accent: #f43f5e;` (bold rose)

---

## Step 4: Deploy

After assembly, deploy with `/vibes:exe`:

```bash
# Standard deployment with registry (REQUIRED)
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" \
  --name wedding-photos \
  --file index.html \
  --clerk-key "$(cat clerk-jwks-key.pem)"

# With AI features enabled
node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" \
  --name wedding-photos \
  --file index.html \
  --clerk-key "$(cat clerk-jwks-key.pem)" \
  --ai-key "sk-or-v1-your-provisioning-key" \
  --multi-tenant \
  --tenant-limit 5
```

**Required Flag:**
- `--clerk-key` - Your Clerk JWKS Public Key (required for registry JWT verification)

**AI Deployment Flags:**
- `--ai-key` - Your OpenRouter provisioning API key
- `--multi-tenant` - Enable per-tenant key provisioning and limits
- `--tenant-limit` - Monthly credit limit per tenant in dollars (default: $5)

Your app will be live at `https://wedding-photos.exe.xyz`

### Verify Registry is Working

After deployment, verify the registry server is running:

```bash
# Should return: {"claims":{},"quotas":{},"reserved":["admin","api","www"],"preallocated":{}}
curl https://wedding-photos.exe.xyz/registry.json
```

If you see HTML instead of JSON, the registry server isn't running. Check the deployment logs or re-run with `--clerk-key`.

If AI is enabled, tenants can use the `useAI` hook and their usage is automatically metered.

For custom domains with wildcard subdomains, see the exe.dev deployment guide.

---

## Step 5: Clerk Setup (REQUIRED BEFORE TESTING)

**⚠️ CRITICAL: Configure Clerk BEFORE testing the deployed app!**

**Read the complete setup guide:** [CLERK-SETUP.md](./CLERK-SETUP.md)

### Quick Checklist

**Email Settings** (Dashboard → User & Authentication → Email):
| Setting | Value | Why |
|---------|-------|-----|
| Sign-up with email | ✅ ON | Users sign up via email |
| Require email address | ⬜ **OFF** | **CRITICAL** - signup fails with "missing_requirements" if ON |
| Verify at sign-up | ✅ ON | Verify before session |
| Email verification code | ✅ Checked | Use code for signup verification |

**Passkey Settings** (Dashboard → User & Authentication → Passkeys):
| Setting | Value | Why |
|---------|-------|-----|
| Sign-in with passkey | ✅ ON | Primary auth method |
| Allow autofill | ✅ ON | Better UX |
| Show passkey button | ✅ ON | Visible option |
| Add passkey to account | ✅ ON | Users can add passkeys |

**Note:** Configure passkey settings AFTER creating your Clerk app (not during initial app creation). The app enforces passkey creation at the application level - Clerk passkeys don't have a "required/optional" setting.

**How authentication works:**
- Signup: email → verify → session active → app forces passkey creation → claim subdomain
- Sign-in: passkey first, email magic link as fallback
- The app blocks access until passkey is created (app-level enforcement)

**If using `--billing-mode required`:**
1. Go to Clerk Dashboard → Billing → Get Started
2. Create subscription plans (names must match: `pro`, `basic`, `monthly`, `yearly`, `starter`, `free`)
3. Set prices and trial periods for each plan
4. Connect your Stripe account

**NO REBUILD REQUIRED**: Clerk setup is done in the Clerk Dashboard only. Change billing mode by re-running assembly with `--billing-mode required` or `--billing-mode off`.

---

## Key Components

### Client-Side Routing

The unified template uses `getRouteInfo()` to detect subdomain and route:

```javascript
function getRouteInfo() {
  const hostname = window.location.hostname;
  const parts = hostname.split('.');
  const params = new URLSearchParams(window.location.search);
  const testSubdomain = params.get('subdomain');

  // Handle localhost testing with ?subdomain= param
  if (hostname === 'localhost' || hostname === '127.0.0.1') {
    if (testSubdomain === 'admin') return { route: 'admin', subdomain: null };
    if (testSubdomain) return { route: 'tenant', subdomain: testSubdomain };
    return { route: 'landing', subdomain: null };
  }

  // Handle exe.xyz testing (before custom domain is set up)
  if (hostname.endsWith('.exe.xyz')) {
    if (testSubdomain === 'admin') return { route: 'admin', subdomain: null };
    if (testSubdomain) return { route: 'tenant', subdomain: testSubdomain };
    return { route: 'landing', subdomain: null };
  }

  // Production: detect subdomain from hostname
  if (parts.length <= 2 || parts[0] === 'www') {
    return { route: 'landing', subdomain: null };
  }
  if (parts[0] === 'admin') {
    return { route: 'admin', subdomain: null };
  }
  return { route: 'tenant', subdomain: parts[0] };
}
```

### TenantContext

Provides database scoping for tenant apps:

```javascript
const TenantContext = createContext(null);

function TenantProvider({ children, subdomain }) {
  const dbName = `${APP_NAME}-${subdomain}`;
  return (
    <TenantContext.Provider value={{ subdomain, dbName, appName: APP_NAME, domain: APP_DOMAIN }}>
      {children}
    </TenantContext.Provider>
  );
}
```

### SubscriptionGate with Billing Mode

The subscription gate respects the billing mode setting:

- **`off`**: Everyone gets free access after signing in
- **`required`**: Users must subscribe via Clerk Billing

Admins always bypass the subscription check.

**SECURITY WARNING**: Do NOT add fallbacks like `|| ADMIN_USER_IDS.length === 0` to admin checks. An empty admin list means NO admin access, not "everyone is admin". The template is secure - do not modify the admin authorization logic.

```javascript
function SubscriptionGate({ children }) {
  const { has, isLoaded, userId } = useAuth();

  if (!isLoaded) {
    return <div>Loading...</div>;
  }

  // Billing mode "off" = everyone gets free access
  if (BILLING_MODE === 'off') {
    return children;
  }

  // Admin bypass
  if (ADMIN_USER_IDS.includes(userId)) {
    return children;
  }

  // Check Clerk Billing subscriptions
  const hasSubscription = has({ plan: 'pro' }) ||
    has({ plan: 'basic' }) ||
    has({ plan: 'monthly' }) ||
    has({ plan: 'yearly' }) ||
    has({ plan: 'starter' }) ||
    has({ plan: 'free' });

  if (!hasSubscription) {
    return <SubscriptionRequired />;
  }

  return children;
}
```

---

## Testing

Test different routes by adding `?subdomain=` parameter:

**Localhost:**
```
http://localhost:5500/index.html              → Landing page
http://localhost:5500/index.html?subdomain=test → Tenant app
http://localhost:5500/index.html?subdomain=admin → Admin dashboard
```

**exe.xyz (before custom domain):**
```
https://myapp.exe.xyz              → Landing page
https://myapp.exe.xyz?subdomain=test → Tenant app
https://myapp.exe.xyz?subdomain=admin → Admin dashboard
```

The `?subdomain=` parameter works on both localhost and exe.xyz, allowing you to test all routes before configuring custom DNS.

---

## Import Map

The unified template uses React 19 with `@necrodome/fireproof-clerk` for Clerk integration:

```json
{
  "imports": {
    "react": "https://esm.sh/stable/react@19.2.4",
    "react/jsx-runtime": "https://esm.sh/stable/react@19.2.4/jsx-runtime",
    "react/jsx-dev-runtime": "https://esm.sh/stable/react@19.2.4/jsx-dev-runtime",
    "react-dom": "https://esm.sh/stable/react-dom@19.2.4",
    "react-dom/client": "https://esm.sh/stable/react-dom@19.2.4/client",
    "use-fireproof": "https://esm.sh/stable/@necrodome/fireproof-clerk@0.0.3?external=react,react-dom",
    "@fireproof/clerk": "https://esm.sh/stable/@necrodome/fireproof-clerk@0.0.3?external=react,react-dom"
  }
}
```

**Note:** Uses `esm.sh/stable/` for pre-built, cached packages. The `?external=react,react-dom` ensures import map controls React.

**IMPORTANT:**
- `@clerk/clerk-react` is imported directly via URL in the code (not via import map) because Babel standalone doesn't properly resolve bare specifiers from import maps.

---

## Troubleshooting

### "Unexpected token '<'" in console
- JSX not being transpiled by Babel
- Check that `<script type="text/babel" data-type="module">` is present
- Verify Babel standalone is loading

### "Cannot read properties of null (reading 'useEffect')"
- React version mismatch between packages
- Ensure fireproof-clerk imports have `?external=react,react-dom`
- Use `esm.sh/stable/` URLs for consistent package resolution

### "Subscription Required" loop
- Check that admin user ID is correct and in the `ADMIN_USER_IDS` array
- Verify Clerk Billing is set up with matching plan names
- Redeploy after updating the file

### Clerk not loading
- Add your domain to Clerk's authorized domains
- Check publishable key is correct (not secret key)
- Verify ClerkProvider wraps the app

### Admin shows "Access Denied"
- User ID not in --admin-ids array
- Check Clerk Dashboard → Users → click user → copy User ID
- Re-run assembly with correct --admin-ids

### Database not isolated
- Verify `useTenant()` is used in the App component
- Check `useFireproofClerk(dbName)` uses the tenant database name

### Passkey creation fails
- Ensure HTTPS is configured (passkeys require secure context)
- Check browser supports WebAuthn (all modern browsers do)
- Verify Passkey settings are enabled in Clerk Dashboard
- Check console for specific error messages
- For production: use `pk_live_*` key and add domain to allowed origins

### "Verification incomplete (missing_requirements)" error
This error during signup means Clerk Email settings are wrong:
1. Go to Clerk Dashboard → User & Authentication → Email
2. Set "Require email address" to **OFF** (this is the critical fix!)
3. Ensure "Sign-up with email" is ON
4. Ensure "Verify at sign-up" is ON with "Email verification code" checked

See [CLERK-SETUP.md](./CLERK-SETUP.md) for complete settings.

---

## What's Next?

After assembly completes, present deployment options using AskUserQuestion:

```
Question: "Your SaaS is assembled! What would you like to do?"
Header: "Next"
Options:
- Label: "Deploy now (/exe)"
  Description: "Go live immediately. Deploy sets up your app at yourapp.exe.xyz with the registry server for subdomain claiming, Clerk authentication, and HTTPS. Tenants can start signing up as soon as it's live."

- Label: "Test locally first"
  Description: "Preview before going live. Open index.html with Live Server and use ?subdomain=test to simulate tenant routing. Test the signup flow, passkey creation, and billing before real users see it."

- Label: "Customize landing page"
  Description: "Fine-tune the marketing. Adjust colors to match your brand, refine the tagline, or update feature descriptions. The landing page is your first impression - make it count."

- Label: "I'm done for now"
  Description: "Wrap up this session. Your index.html is ready to deploy whenever you're ready - just run /vibes:exe later."
```

**After user responds:**
- "Deploy now" → Auto-invoke /vibes:exe skill with these flags:
  - `--name` from app-name
  - `--file index.html`
  - `--clerk-key "JWKS_KEY"` (always required for registry)
  - Optionally: `--clerk-webhook-secret "SECRET"` for subscription sync
  - If AI enabled: `--ai-key "OPENROUTER_KEY"` and `--multi-tenant`
- "Test locally" → Provide localhost testing instructions with ?subdomain= params
- "Customize" → Stay ready for customization prompts
- "I'm done" → Confirm index.html saved, remind of deploy command with all flags needed

### After Deploy: Configure Admin Access

After the first deployment, guide the user through admin setup:

```
Question: "Your app is live! Ready to set up admin access?"
Header: "Admin"
Options:
- Label: "Set up admin now (Recommended)"
  Description: "Sign up on your app, get your user ID from Clerk Dashboard, then re-deploy with admin access."

- Label: "Skip for now"
  Description: "You can add admin access later by re-running assembly with --admin-ids."
```

**If "Set up admin now" is selected:**
1. Tell user to visit their app and sign up (e.g., `https://myapp.exe.xyz`)
2. After signup, go to Clerk Dashboard → Users → click their user → copy User ID
3. Re-run assembly with `--admin-ids '["user_xxx"]'`
4. Re-deploy with the same flags

This ensures the admin dashboard is accessible from the start.
