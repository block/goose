#!/usr/bin/env node
/**
 * Sell App Assembler
 *
 * Assembles a SaaS app from the sell template and user's app code.
 * Creates a client-side only app - no backend server needed.
 *
 * Creates:
 *   - index.html - Unified app handling landing, tenant, and admin routes
 *
 * Usage:
 *   node scripts/assemble-sell.js <app.jsx> [output.html] [options]
 *
 * Options:
 *   --clerk-key <key>     Clerk publishable key (required)
 *   --app-name <name>     App name for database naming (e.g., "wedding-photos")
 *   --app-title <title>   Display title (e.g., "Wedding Photos")
 *   --domain <domain>     Root domain (e.g., "myapp.exe.xyz")
 *   --billing-mode <mode> Billing mode: "off" (free) or "required" (subscription required)
 *   --features <json>     JSON array of feature strings
 *   --tagline <text>      App tagline for landing page
 *   --admin-ids <json>    JSON array of Clerk user IDs with admin access
 *   --reserved <csv>      Comma-separated reserved subdomain names
 *
 * Example:
 *   node scripts/assemble-sell.js app.jsx index.html \
 *     --clerk-key pk_test_xxx \
 *     --app-name wedding-photos \
 *     --app-title "Wedding Photos" \
 *     --domain myapp.exe.xyz \
 *     --admin-ids '["user_xxx"]'
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve } from 'path';
import { TEMPLATES } from './lib/paths.js';
import { stripForTemplate, stripImports } from './lib/strip-code.js';
import { createBackup } from './lib/backup.js';
import { prompt } from './lib/prompt.js';

// Parse command line arguments
function parseArgs(argv) {
  const args = {
    appJsxPath: null,
    outputPath: null,
    options: {}
  };

  let i = 2;
  while (i < argv.length) {
    const arg = argv[i];

    if (arg.startsWith('--')) {
      const key = arg.slice(2).replace(/-([a-z])/g, (_, c) => c.toUpperCase());
      const value = argv[i + 1];
      args.options[key] = value;
      i += 2;
    } else if (!args.appJsxPath) {
      args.appJsxPath = arg;
      i++;
    } else if (!args.outputPath) {
      args.outputPath = arg;
      i++;
    } else {
      i++;
    }
  }

  return args;
}

const { appJsxPath, outputPath, options } = parseArgs(process.argv);

// Validate app.jsx path
if (!appJsxPath) {
  console.error('Usage: node scripts/assemble-sell.js <app.jsx> [output.html] [options]');
  console.error('\nProvide the path to your app.jsx file.');
  console.error('Run with no arguments to see full usage.');
  process.exit(1);
}

const resolvedAppPath = resolve(appJsxPath);
if (!existsSync(resolvedAppPath)) {
  console.error(`App file not found: ${resolvedAppPath}`);
  process.exit(1);
}

// Default output path
const resolvedOutputPath = resolve(outputPath || 'index.html');

// Backup existing index.html if it exists
const sellBackupPath = createBackup(resolvedOutputPath);
if (sellBackupPath) {
  console.log(`Backed up existing file to: ${sellBackupPath}`);
}

// Template paths (centralized in lib/paths.js)
const templatePath = TEMPLATES.sellUnified;
const adminComponentPath = TEMPLATES.adminComponent;

// Check templates exist
const templateChecks = [
  { path: templatePath, name: 'unified.html' },
  { path: adminComponentPath, name: 'components/admin-exe.jsx' }
];

for (const t of templateChecks) {
  if (!existsSync(t.path)) {
    console.error(`Template not found: ${t.path}`);
    console.error('Make sure the sell skill templates are installed.');
    process.exit(1);
  }
}

// Read template
let output = readFileSync(templatePath, 'utf8');

// Configuration - prompt for domain if not provided
let domain = options.domain;
if (!domain) {
  console.log('\n⚠ No --domain provided. This is required for SaaS apps.\n');
  domain = await prompt('Enter domain (e.g., myapp.exe.xyz): ');
  if (!domain) {
    console.error('Error: Domain is required. Provide via --domain or enter when prompted.');
    process.exit(1);
  }
}
const appName = options.appName || 'my-app';

// Configuration replacements
const replacements = {
  '__CLERK_PUBLISHABLE_KEY__': options.clerkKey || 'pk_test_YOUR_KEY_HERE',
  '__APP_NAME__': appName,
  '__APP_TITLE__': options.appTitle || appName,
  '__APP_DOMAIN__': domain,
  '__BILLING_MODE__': options.billingMode || 'off',
  '__APP_TAGLINE__': options.tagline || 'Your own private workspace. Get started in seconds.'
};

// Handle JSON values - features
if (options.features) {
  try {
    const features = JSON.parse(options.features);
    replacements['__FEATURES__'] = JSON.stringify(features);
  } catch (e) {
    console.warn('Warning: Could not parse --features as JSON, using default');
    replacements['__FEATURES__'] = '["Unlimited usage", "Private workspace", "Custom subdomain"]';
  }
} else {
  replacements['__FEATURES__'] = '["Unlimited usage", "Private workspace", "Custom subdomain"]';
}

// Handle JSON values - admin IDs
if (options.adminIds) {
  try {
    const adminIds = JSON.parse(options.adminIds);
    replacements['__ADMIN_USER_IDS__'] = JSON.stringify(adminIds);
  } catch (e) {
    console.warn('Warning: Could not parse --admin-ids as JSON, using empty array');
    replacements['__ADMIN_USER_IDS__'] = '[]';
  }
} else {
  replacements['__ADMIN_USER_IDS__'] = '[]';
}

// Handle reserved subdomain names (comma-separated)
if (options.reserved) {
  const reserved = options.reserved.split(',').map(s => s.trim().toLowerCase()).filter(Boolean);
  replacements['__RESERVED_SUBDOMAINS__'] = JSON.stringify(reserved);
} else {
  replacements['__RESERVED_SUBDOMAINS__'] = '[]';
}

// Apply replacements
for (const [placeholder, value] of Object.entries(replacements)) {
  output = output.split(placeholder).join(value);
}

// Read and process app code - strip imports, exports, and template-provided constants
const templateConstants = ['CLERK_PUBLISHABLE_KEY', 'APP_NAME', 'APP_DOMAIN', 'BILLING_MODE', 'FEATURES', 'APP_TAGLINE', 'ADMIN_USER_IDS'];
let appCode = stripForTemplate(readFileSync(resolvedAppPath, 'utf8'), templateConstants);

// Check if app uses hardcoded database name
const firepoolMatch = appCode.match(/useFireproof\s*\(\s*["']([^"']+)["']\s*\)/);
if (firepoolMatch) {
  const originalDbName = firepoolMatch[1];
  console.log(`Note: Found hardcoded database name "${originalDbName}".`);
  console.log('      The unified template uses dynamic database naming via useTenant().dbName');
  console.log('      You may need to update your App component to use: const { dbName } = useTenant();');
}

// Insert app code at placeholder
const appPlaceholder = '__VIBES_APP_CODE__';
if (output.includes(appPlaceholder)) {
  output = output.replace(appPlaceholder, appCode);
} else {
  console.error(`Template missing placeholder: ${appPlaceholder}`);
  process.exit(1);
}

// Read and process admin component - strip imports (template already imports dependencies)
let adminCode = stripImports(readFileSync(adminComponentPath, 'utf8').trim());

// Insert admin code at placeholder
const adminPlaceholder = '__ADMIN_CODE__';
if (output.includes(adminPlaceholder)) {
  output = output.replace(adminPlaceholder, adminCode);
} else {
  console.error(`Template missing placeholder: ${adminPlaceholder}`);
  process.exit(1);
}

// Known safe patterns that aren't config placeholders
// __PURE__ is a tree-shaking comment used by bundlers
// __esModule is used by transpilers for ES module compatibility
const SAFE_PLACEHOLDER_PATTERNS = ['__PURE__', '__esModule'];

// Validate output
function validateSellAssembly(html, app, admin) {
  const errors = [];

  if (!app || app.trim().length === 0) {
    errors.push('App code is empty');
  }

  if (!admin || admin.trim().length === 0) {
    errors.push('Admin code is empty');
  }

  // Check for unreplaced placeholders using whitelist approach
  const allMatches = html.match(/__[A-Z_]+__/g) || [];
  const unreplaced = allMatches.filter(m => !SAFE_PLACEHOLDER_PATTERNS.includes(m));
  if (unreplaced.length > 0) {
    errors.push(`Unreplaced placeholders: ${[...new Set(unreplaced)].join(', ')}`);
  }

  if (!html.includes('export default function') && !html.includes('function App')) {
    errors.push('No App component found');
  }

  return errors;
}

const validationErrors = validateSellAssembly(output, appCode, adminCode);
if (validationErrors.length > 0) {
  console.error('Sell assembly failed:');
  validationErrors.forEach(e => console.error(`  - ${e}`));
  console.error('\nFix: Check your app.jsx and admin component for errors');
  process.exit(1);
}

// Write main output
writeFileSync(resolvedOutputPath, output);
console.log(`\n✓ Created: ${resolvedOutputPath}`);

// Print deployment guide
console.log(`
══════════════════════════════════════════════════════════════════
  ${appName.toUpperCase()} - DEPLOYMENT GUIDE
══════════════════════════════════════════════════════════════════

This is a client-side only SaaS app. No backend server required.

STEP 1: DEPLOY TO exe.dev
─────────────────────────

  Run /vibes:exe to deploy to exe.dev, or manually:

  node "\${CLAUDE_PLUGIN_ROOT}/scripts/deploy-exe.js" --name ${appName} --file index.html

  Your app will be live at: https://${appName}.exe.xyz

STEP 2: SET UP CLERK (REQUIRED BEFORE TESTING)
───────────────────────────────────────────────

  See CLERK-SETUP.md for complete instructions. Critical settings:

  Dashboard → User & Authentication → Email:
    ✅ Sign-up with email: ON
    ⚠️  Require email address: OFF (critical - signup fails otherwise!)
    ✅ Verify at sign-up: ON
    ✅ Email verification code: CHECKED

  Dashboard → User & Authentication → Passkeys:
    ✅ Sign-in with passkey: ON
    ✅ Allow autofill: ON
    ✅ Show passkey button: ON
    ✅ Add passkey to account: ON

  Dashboard → Domains:
    Add your domain (e.g., ${domain})

  Get your Publishable Key and re-run assembly:

     node assemble-sell.js app.jsx index.html \\
       --clerk-key pk_live_YOUR_KEY \\
       --app-name ${appName} \\
       --domain ${domain}

STEP 3: SET UP WILDCARD DNS (Optional - for subdomains)
───────────────────────────────────────────────────────

  For tenant subdomains (e.g., alice.${domain}), you need:

  1. Custom domain pointing to your exe.dev VM
  2. Wildcard DNS: *.${domain} → VM IP
  3. Wildcard SSL certificate (via certbot DNS-01)

  See exe.dev docs for wildcard SSL setup.

STEP 4: CONFIGURE BILLING (if --billing-mode required)
───────────────────────────────────────────────────────

  1. Go to Clerk Dashboard → Billing → Get Started
  2. Create subscription plans (pro, basic, monthly, yearly, starter, free)
  3. Connect your Stripe account
  4. Re-run assembly with --billing-mode required:

     node assemble-sell.js app.jsx index.html \\
       --billing-mode required \\
       ... other options

  See CLERK-SETUP.md for detailed billing configuration.

WHAT WORKS
──────────
  ✓ Landing page with subdomain claim
  ✓ Clerk authentication (passkeys)
  ✓ Tenant app with database isolation
  ✓ Admin dashboard (config view only)
  ✓ Subscription gating via Clerk Billing

══════════════════════════════════════════════════════════════════
`);
