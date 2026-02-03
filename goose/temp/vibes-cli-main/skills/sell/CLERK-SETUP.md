# Clerk Setup Guide

This document provides step-by-step instructions for configuring Clerk after deployment.

**NO REBUILD REQUIRED**: Clerk setup is done in the Clerk Dashboard only. The app already has Clerk integration built in - you just need to configure your Clerk account. Do NOT regenerate or reassemble the app.

---

## 1. Create Clerk Application

1. Go to [clerk.com](https://clerk.com) and sign in
2. Create a new application
3. Copy the **Publishable Key** (pk_test_... or pk_live_...)

---

## 2. Configure Authentication Settings

The sell template uses passkey-first authentication. **Follow these settings exactly:**

### 2.1 Email Settings

In Clerk Dashboard → **User & Authentication** → **Email**:

| Setting | Value | Why |
|---------|-------|-----|
| Sign-up with email | ✅ ON | Users sign up via email |
| Require email address | ⬜ OFF | **IMPORTANT**: Must be OFF or signup fails |
| Verify at sign-up | ✅ ON | Verify before session |
| Email verification code | ✅ Checked | Use code, not magic link for signup |

### 2.2 Passkey Settings

**Note:** Configure these settings AFTER creating your Clerk app, not during the initial app creation.

In Clerk Dashboard → **User & Authentication** → **Passkeys**:

| Setting | Value | Why |
|---------|-------|-----|
| Sign-in with passkey | ✅ ON | Primary auth method |
| Allow autofill | ✅ ON | Better UX |
| Show passkey button | ✅ ON | Visible option |
| Add passkey to account | ✅ ON | Users can add passkeys |

**Note:** The app enforces passkey creation at the application level, not via Clerk settings. Clerk passkeys don't have an "optional/required" setting - the app handles this.

### Passkey Scope for Subdomains

Passkeys created on one subdomain (e.g., `alice.yourapp.com`) will work across all subdomains of your domain because WebAuthn uses the root domain as the RP ID (Relying Party ID). A single passkey gives access to all claimed subdomains.

**Reference:** [Clerk Custom Passkey Authentication](https://clerk.com/docs/guides/development/custom-flows/authentication/passkeys)

### 2.3 Get Your JWKS Public Key (for Registry Server)

The registry server needs Clerk's public key to verify JWT tokens. **Important:** The key shown in Clerk Dashboard may differ from the JWKS endpoint. Always use the JWKS endpoint.

1. Find your Clerk domain from the publishable key:
   - `pk_test_abc123...` → decode base64 to get domain like `internal-dingo-28.clerk.accounts.dev`

2. Fetch the JWKS:
   ```bash
   curl https://YOUR_CLERK_DOMAIN/.well-known/jwks.json
   ```

3. Convert JWK to PEM using Node.js:
   ```javascript
   const crypto = require('crypto');
   const jwk = { /* paste the key object from JWKS */ };
   const key = crypto.createPublicKey({ key: jwk, format: 'jwk' });
   console.log(key.export({ type: 'spki', format: 'pem' }));
   ```

4. Save as `/etc/clerk-public-key.pem` on the server

---

## 3. Get Your Admin User ID

1. Sign up on your app (or use an existing Clerk account)
2. Go to Clerk Dashboard → **Users**
3. Click your user
4. Copy the **User ID** (e.g., `user_2abc123xyz`)

Use this ID in the `--admin-ids` flag when running the assembly script.

---

## 4. Subdomain Registry & Webhooks

The sell template uses a server-side registry to track subdomain ownership. This enables:
- First-claim ownership (first user to claim a subdomain owns it)
- Unlimited subdomains per user (each must be paid for)
- Immediate release when subscription lapses

### 4.1 Configure Webhook Endpoint

The registry server listens for Clerk subscription webhooks to release subdomains when subscriptions change.

1. In Clerk Dashboard, go to **Webhooks**
2. Click **Add Endpoint**
3. Enter your webhook URL: `https://yourapp.exe.xyz/webhook`
4. Select these events:
   - `subscription.updated`
   - `subscription.deleted`
5. Copy the **Signing Secret**

### 4.2 Set Webhook Secret

The webhook secret is set when deploying:

```bash
node deploy-exe.js --name myapp \
  --file index.html \
  --clerk-webhook-secret whsec_xxxxx
```

Or set it in `/etc/registry.env` on the VM:
```
CLERK_WEBHOOK_SECRET=whsec_xxxxx
```

Then restart the registry service:
```bash
sudo systemctl restart registry
```

---

## 5. Enable Clerk Billing (Required for `--billing-mode required`)

If your app uses `--billing-mode required`, you must configure Clerk Billing:

### 5.1 Navigate to Billing
1. In Clerk Dashboard, go to **Billing**
2. Click **Get Started** if this is your first time

### 5.2 Create Subscription Plans with Quantity

For multi-subdomain support, create a plan with **quantity** enabled:

1. Go to **Billing** → **Plans**
2. Create a plan (e.g., `subdomain-access`)
3. Enable **Per-seat pricing** or **Usage-based pricing**
4. Set the price per subdomain (e.g., $5/month per subdomain)

The registry server automatically updates subscription quantity when users claim or lose subdomains.

### 5.3 Alternative: Simple Plans

If you prefer simple subscription tiers:
- `pro` - Premium tier (unlimited subdomains)
- `basic` - Entry tier (limited subdomains)
- `monthly` - Monthly subscription
- `yearly` - Annual subscription
- `starter` - Starter tier
- `free` - Free tier with limited features

**Important:** Plan names are case-sensitive and must match exactly.

### 5.4 Configure Plan Details
For each plan:
1. Set the price (e.g., $9/month, $89/year)
2. Add features that will display in the PricingTable
3. Configure trial period if desired (e.g., 14 days free)

### 5.5 Connect Stripe
1. Go to **Billing** → **Stripe**
2. Click **Connect Stripe Account**
3. Complete the Stripe onboarding flow
4. For testing, use Stripe test mode

---

## 6. Understanding Billing Modes

Your app supports two billing modes set during assembly:

### billing-mode: off (default)
- Everyone gets free access after signing in
- No subscription required
- Good for testing or free apps
- PricingTable not shown on landing page

### billing-mode: required
- Users must subscribe to access tenant app
- PricingTable shown on landing page
- Non-subscribed users see paywall with PricingTable
- Admins always bypass billing gate
- Trial periods configured in Clerk Dashboard

To change billing mode:
```bash
node assemble-sell.js app.jsx index.html \
  --clerk-key pk_live_xxx \
  --billing-mode required \
  ... (other options)
```

---

## 7. Reserved Subdomains

Operators can reserve subdomains at deploy time to prevent users from claiming them:

```bash
node deploy-exe.js --name myapp \
  --file index.html \
  --reserved "admin,billing,api,www,support"
```

Reserved subdomains show as "unavailable" to users.

### Pre-allocated Subdomains

For enterprise customers or special cases, pre-allocate subdomains to specific user IDs:

```bash
node deploy-exe.js --name myapp \
  --file index.html \
  --preallocated "acme:user_enterprise1,bigcorp:user_enterprise2"
```

Pre-allocated subdomains are immediately owned by the specified user.

---

## 8. Testing

### Test Mode
1. Use Clerk test keys (`pk_test_xxx`)
2. Enable Stripe test mode in Clerk Dashboard
3. Use Stripe test cards: `4242 4242 4242 4242`

### Verify Subdomain Claiming
1. Visit an unclaimed subdomain (e.g., `test.yourapp.com`)
2. You should see the passkey signup flow
3. Complete signup - subdomain should be claimed
4. Check the registry: `curl https://yourapp.com/registry.json`

### Verify Billing Flow
1. Visit your landing page
2. Click a plan in the PricingTable
3. Complete checkout with test card
4. Visit your subdomain - should have access

### Verify Paywall
1. Create new user without subscription
2. Visit subdomain
3. Should see paywall with PricingTable
4. Subscribe via paywall
5. Should redirect to app with access

### Verify Webhook
1. Cancel a subscription in Clerk/Stripe
2. Check registry - subdomain should be released
3. Visit the subdomain - should show as available

---

## Clerk Setup Checklist

### Basic Setup
- [ ] Create Clerk application and get publishable key
- [ ] Get JWKS public key (from `/.well-known/jwks.json`, NOT Dashboard)
- [ ] Configure Email settings (after app creation):
  - [ ] Sign-up with email: ON
  - [ ] Require email address: **OFF** (critical!)
  - [ ] Verify at sign-up: ON
  - [ ] Email verification code: Checked
- [ ] Configure Passkey settings (after app creation):
  - [ ] Sign-in with passkey: ON
  - [ ] Allow autofill: ON
  - [ ] Show passkey button: ON
  - [ ] Add passkey to account: ON

### After First Deploy
- [ ] Sign up on your app to become the first user
- [ ] Get admin user ID from Clerk Dashboard → Users
- [ ] Re-run assembly with `--admin-ids '["user_xxx"]'`
- [ ] Re-deploy to enable admin dashboard access
- [ ] Verify registry: `curl https://yourapp.exe.xyz/registry.json`

### Subdomain Registry
- [ ] Configure webhook endpoint in Clerk Dashboard
- [ ] Set webhook secret in registry server

### Billing (if using `--billing-mode required`)
- [ ] Enable Clerk Billing
- [ ] Create subscription plans (with quantity for multi-subdomain)
- [ ] Connect Stripe account

---

## How Data Works

### Subdomain Registry (Server-side)

Subdomain ownership is stored in `/var/www/html/registry.json`:

```json
{
  "claims": {
    "alice": { "userId": "user_abc123", "claimedAt": "2025-01-04T..." }
  },
  "reserved": ["admin", "billing", "api"],
  "preallocated": {
    "enterprise": "user_xyz789"
  }
}
```

### User Metadata (Client-side)

Additional user data is stored in Clerk's `unsafeMetadata`:
- **plan**: User's subscription plan name
- **registeredAt**: Registration timestamp

### Subscription Quantity

For multi-subdomain users, the number of claimed subdomains equals their subscription quantity. When quantity decreases (downgrade or cancellation), subdomains are released in LIFO order (newest first).

---

## Troubleshooting

### Registry fetch failed error
If you see "Registry fetch failed, using empty defaults" in the console:
1. Verify registry server is running: `sudo systemctl status vibes-registry`
2. Check registry endpoint: `curl https://yourapp.exe.xyz/registry.json`
3. Verify `--clerk-key` was provided during deployment
4. Check registry logs: `sudo journalctl -u vibes-registry -f`
5. If you see HTML instead of JSON, nginx may not be proxying to the registry server

### Clerk not loading
1. Check publishable key is correct (use pk_test_... for dev, pk_live_... for production)
2. Verify the domain matches your deployment URL

### Passkey creation fails
1. Ensure HTTPS is configured (passkeys require secure context)
2. Check browser supports WebAuthn (all modern browsers do)
3. Verify Passkey is enabled in Clerk Dashboard

### "Verification incomplete (missing_requirements)" error
This error during signup means Clerk Email settings are wrong:
1. Go to Clerk Dashboard → User & Authentication → Email
2. Set "Require email address" to **OFF**
3. Ensure "Sign-up with email" is ON
4. Ensure "Verify at sign-up" is ON with "Email verification code" checked

### Subdomain shows "unavailable" but shouldn't be
1. Check `/registry.json` - is it in `reserved` or `claims`?
2. Verify registry server is running: `sudo systemctl status registry`
3. Check nginx proxy config is correct

### Webhook not releasing subdomains
1. Verify webhook URL is correct in Clerk Dashboard
2. Check webhook secret matches `/etc/registry.env`
3. View registry logs: `sudo journalctl -u registry -f`
4. Verify webhook events are selected (subscription.updated, subscription.deleted)

### Billing not working
1. Verify plan names in Clerk match what your app checks
2. Ensure Clerk Billing is enabled in your account
3. Check that `has({ plan: 'yourplan' })` matches the plan name exactly

### "Access Denied" on admin dashboard
1. Verify your user ID is in the `--admin-ids` array
2. Re-run assembly with the correct admin IDs
3. Redeploy the updated index.html

---

## Reference Documentation

- [Clerk Custom Flows Overview](https://clerk.com/docs/guides/development/custom-flows/overview)
- [Clerk Custom Passkey Authentication](https://clerk.com/docs/guides/development/custom-flows/authentication/passkeys)
- [WebAuthn RP ID Best Practices](https://www.corbado.com/blog/webauthn-relying-party-id-rpid-passkeys)
