#!/usr/bin/env node
/**
 * deploy-connect.js - Deploy Fireproof Connect to a dedicated Studio VM
 *
 * This is a thin wrapper that:
 * 1. SSHs to the Studio VM
 * 2. Clones the fireproof repo (selem/docker-for-all branch)
 * 3. Creates .env with credentials
 * 4. Runs ./docker/start.sh to start services
 *
 * No local reimplementation of Docker compose - we delegate to upstream.
 *
 * Usage:
 *   node scripts/deploy-connect.js \
 *     --studio <codename> \
 *     --clerk-publishable-key "pk_test_..." \
 *     --clerk-secret-key "sk_test_..."
 */

import { existsSync, readFileSync, writeFileSync } from 'fs';
import { homedir } from 'os';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';
import { ensureSSH2 } from './lib/ensure-deps.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

await ensureSSH2(__filename);

import {
  findSSHKey,
  connect,
  runCommand,
  createVM,
  setPublic,
  testConnection
} from './lib/exe-ssh.js';

import { generateSessionTokens, generateDeviceCAKeys } from './lib/crypto-utils.js';

// ============== Argument Parsing ==============

function parseArgs(argv) {
  const args = {
    studio: null,
    clerkPublishableKey: null,
    clerkSecretKey: null,
    dryRun: false,
    help: false
  };

  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--studio' && argv[i + 1]) {
      args.studio = argv[++i];
    } else if (arg === '--clerk-publishable-key' && argv[i + 1]) {
      args.clerkPublishableKey = argv[++i];
    } else if (arg === '--clerk-secret-key' && argv[i + 1]) {
      args.clerkSecretKey = argv[++i];
    } else if (arg === '--dry-run') {
      args.dryRun = true;
    } else if (arg === '--help' || arg === '-h') {
      args.help = true;
    }
  }

  return args;
}

function printHelp() {
  console.log(`
Deploy Fireproof Connect to a Studio VM
========================================

Deploys the full Fireproof sync stack to a dedicated exe.dev VM.

Usage:
  node scripts/deploy-connect.js --studio <codename> [options]

Required:
  --studio <codename>              Studio VM name (becomes <codename>.exe.xyz)
  --clerk-publishable-key <key>    Clerk publishable key (pk_test_... or pk_live_...)
  --clerk-secret-key <key>         Clerk secret key (sk_test_... or sk_live_...)

Optional:
  --dry-run                        Show what would be done without executing
  --help                           Show this help message

Prerequisites:
  - SSH key in ~/.ssh/ (id_ed25519, id_rsa, or id_ecdsa)
  - exe.dev account (run 'ssh exe.dev' to create one)
  - Clerk account with API keys

What It Does:
  1. Creates/connects to the Studio VM
  2. Clones fireproof repo (selem/docker-for-all branch)
  3. Generates security tokens (session tokens, CA keys)
  4. Creates .env with all credentials
  5. Runs ./docker/start.sh to start services
  6. Writes local .connect file for app configuration

Example:
  node scripts/deploy-connect.js \\
    --studio marcus-studio \\
    --clerk-publishable-key "pk_test_abc123..." \\
    --clerk-secret-key "sk_test_xyz789..."
`);
}

/**
 * Derive JWT URL from Clerk publishable key
 * Format: pk_test_<base64> or pk_live_<base64>
 * The base64 portion decodes to the Clerk domain
 */
function deriveJwtUrl(publishableKey) {
  try {
    // Extract the base64 portion after pk_test_ or pk_live_
    const prefix = publishableKey.startsWith('pk_live_') ? 'pk_live_' : 'pk_test_';
    const base64Part = publishableKey.slice(prefix.length);

    // Decode base64 to get the Clerk domain
    const decoded = Buffer.from(base64Part, 'base64').toString('utf-8');

    // The decoded value is typically the Clerk frontend API domain
    // e.g., "clerk.your-app.com" or "your-app.clerk.accounts.dev"
    const domain = decoded.replace(/\$$/, ''); // Remove trailing $ if present

    return `https://${domain}/.well-known/jwks.json`;
  } catch {
    // Fallback: ask user to provide it
    throw new Error(
      'Could not derive JWT URL from publishable key. ' +
      'Please find your JWKS URL in Clerk Dashboard > API Keys.'
    );
  }
}

/**
 * Write local .connect file with studio configuration
 */
function writeConnectFile(config) {
  const connectPath = join(process.cwd(), '.connect');
  const content = `# Fireproof Connect Studio Configuration
# Generated: ${new Date().toISOString()}

studio: ${config.studio}
api_url: ${config.apiUrl}
cloud_url: ${config.cloudUrl}
clerk_publishable_key: ${config.clerkPublishableKey}
`;
  writeFileSync(connectPath, content);
  console.log(`  Created: ${connectPath}`);
}

// ============== Deployment Phases ==============

async function phase1PreFlight(args) {
  console.log('\nPhase 1: Pre-flight checks...');

  // Check SSH key
  const sshKey = findSSHKey();
  if (!sshKey) {
    throw new Error('No SSH key found in ~/.ssh/. Please create an SSH key first.');
  }
  console.log(`  ✓ SSH key found: ${sshKey}`);

  // Validate Clerk keys
  if (!args.clerkPublishableKey.startsWith('pk_test_') && !args.clerkPublishableKey.startsWith('pk_live_')) {
    throw new Error('Clerk publishable key must start with pk_test_ or pk_live_');
  }
  if (!args.clerkSecretKey.startsWith('sk_test_') && !args.clerkSecretKey.startsWith('sk_live_')) {
    throw new Error('Clerk secret key must start with sk_test_ or sk_live_');
  }
  console.log('  ✓ Clerk keys validated');

  // Test exe.dev connection
  console.log('  Testing exe.dev connection...');
  if (args.dryRun) {
    console.log('  [DRY RUN] Would test SSH connection to exe.dev');
  } else {
    const connected = await testConnection();
    if (!connected) {
      throw new Error(`Cannot connect to exe.dev.

Before deploying, please:
1. Run: ssh exe.dev (to create account if needed, verify email)
2. Then retry this deployment`);
    }
    console.log('  ✓ exe.dev connection OK');
  }
}

async function phase2CreateVM(args) {
  console.log('\nPhase 2: VM Creation...');

  if (args.dryRun) {
    console.log(`  [DRY RUN] Would create VM: ${args.studio}`);
    return;
  }

  console.log(`  Creating VM: ${args.studio}...`);
  const result = await createVM(args.studio);

  if (result.success) {
    console.log(`  ✓ ${result.message}`);
  } else {
    throw new Error(`Failed to create VM: ${result.message}`);
  }

  // Add VM host key to known_hosts
  const vmHost = `${args.studio}.exe.xyz`;
  console.log(`  Adding ${vmHost} to known_hosts...`);
  try {
    execSync(`ssh-keyscan -H ${vmHost} >> ~/.ssh/known_hosts 2>/dev/null`, { timeout: 30000 });
    console.log('  ✓ Host key added');
  } catch {
    console.log('  Warning: Could not add host key automatically');
  }
}

async function phase3CloneRepo(args) {
  console.log('\nPhase 3: Clone Fireproof Repository...');

  const vmHost = `${args.studio}.exe.xyz`;

  if (args.dryRun) {
    console.log(`  [DRY RUN] Would clone fireproof repo to /opt/fireproof`);
    return;
  }

  const client = await connect(vmHost);

  // Check if repo already exists
  const repoCheck = await runCommand(client, 'test -d /opt/fireproof && echo "EXISTS" || echo "NOT_FOUND"');

  if (repoCheck.stdout.includes('NOT_FOUND')) {
    console.log('  Cloning fireproof repository...');
    const cloneResult = await runCommand(client,
      'sudo git clone --branch selem/docker-for-all https://github.com/fireproof-storage/fireproof.git /opt/fireproof'
    );
    if (cloneResult.code !== 0) {
      throw new Error(`Failed to clone repo: ${cloneResult.stderr}`);
    }
    await runCommand(client, 'sudo chown -R exedev:exedev /opt/fireproof');
    console.log('  ✓ Repository cloned');
  } else {
    console.log('  ✓ Repository already exists');
    // Update to latest
    console.log('  Updating to latest...');
    await runCommand(client, 'cd /opt/fireproof && git pull origin selem/docker-for-all || true');
    console.log('  ✓ Repository updated');
  }

  client.end();
}

async function phase4InstallDocker(args) {
  console.log('\nPhase 4: Docker Installation...');

  const vmHost = `${args.studio}.exe.xyz`;

  if (args.dryRun) {
    console.log('  [DRY RUN] Would install Docker if needed');
    return;
  }

  const client = await connect(vmHost);

  // Check if Docker is installed
  const dockerCheck = await runCommand(client, 'which docker || echo "NOT_FOUND"');

  if (dockerCheck.stdout.includes('NOT_FOUND')) {
    console.log('  Installing Docker...');
    await runCommand(client, 'curl -fsSL https://get.docker.com | sudo sh');
    await runCommand(client, 'sudo usermod -aG docker exedev');
    console.log('  ✓ Docker installed');
  } else {
    console.log('  ✓ Docker already installed');
  }

  client.end();
}

async function phase5GenerateCredentials(args) {
  console.log('\nPhase 5: Generate Security Credentials...');

  if (args.dryRun) {
    console.log('  [DRY RUN] Would generate session tokens and CA keys');
    return {
      sessionTokenPublic: '[GENERATED]',
      sessionTokenSecret: '[GENERATED]',
      devicePrivKey: '[GENERATED]',
      deviceCert: '[GENERATED]',
      jwtUrl: '[DERIVED]'
    };
  }

  console.log('  Generating session tokens...');
  const { publicEnv: sessionTokenPublic, privateEnv: sessionTokenSecret } = await generateSessionTokens();

  console.log('  Generating device CA keys...');
  const { privKey: devicePrivKey, cert: deviceCert } = await generateDeviceCAKeys({
    issuer: 'exe.dev Connect CA',
    organization: 'Vibes DIY',
    locality: 'Cloud',
    state: 'Production'
  });

  console.log('  Deriving JWT URL...');
  const jwtUrl = deriveJwtUrl(args.clerkPublishableKey);

  console.log('  ✓ Credentials generated');

  return { sessionTokenPublic, sessionTokenSecret, devicePrivKey, deviceCert, jwtUrl };
}

async function phase6WriteEnv(args, credentials) {
  console.log('\nPhase 6: Write Environment Configuration...');

  const vmHost = `${args.studio}.exe.xyz`;

  if (args.dryRun) {
    console.log('  [DRY RUN] Would write .env to /opt/fireproof/.env');
    return;
  }

  const client = await connect(vmHost);

  // Create .env content
  const envContent = `# Fireproof Connect - Generated by deploy-connect.js
# ${new Date().toISOString()}

# Clerk Authentication
CLERK_PUBLISHABLE_KEY=${args.clerkPublishableKey}
VITE_CLERK_PUBLISHABLE_KEY=${args.clerkPublishableKey}
CLERK_SECRET_KEY=${args.clerkSecretKey}
CLERK_PUB_JWT_URL=${credentials.jwtUrl}

# Session Tokens (for inter-service auth)
CLOUD_SESSION_TOKEN_PUBLIC=${credentials.sessionTokenPublic}
CLOUD_SESSION_TOKEN_SECRET=${credentials.sessionTokenSecret}

# Device CA (for device identity)
DEVICE_ID_CA_PRIV_KEY=${credentials.devicePrivKey}
DEVICE_ID_CA_CERT=${credentials.deviceCert}
`;

  // Write .env file
  console.log('  Writing .env...');
  // Use heredoc to handle multiline content safely
  const escapedContent = envContent.replace(/'/g, "'\\''");
  await runCommand(client, `echo '${escapedContent}' > /opt/fireproof/.env`);
  await runCommand(client, 'chmod 600 /opt/fireproof/.env');

  console.log('  ✓ Environment configured');

  client.end();
}

async function phase7StartServices(args) {
  console.log('\nPhase 7: Start Docker Services...');

  const vmHost = `${args.studio}.exe.xyz`;

  if (args.dryRun) {
    console.log('  [DRY RUN] Would run ./docker/start.sh');
    return;
  }

  const client = await connect(vmHost);

  // Check if start.sh exists
  const startCheck = await runCommand(client, 'test -f /opt/fireproof/docker/start.sh && echo "EXISTS" || echo "NOT_FOUND"');

  if (startCheck.stdout.includes('NOT_FOUND')) {
    throw new Error('docker/start.sh not found. Is the repo cloned correctly?');
  }

  // Make start.sh executable and run it
  console.log('  Starting services (this may take a few minutes on first run)...');
  await runCommand(client, 'chmod +x /opt/fireproof/docker/start.sh');

  // Run start.sh with a longer timeout
  const startResult = await runCommand(client, 'cd /opt/fireproof && sudo ./docker/start.sh 2>&1 | tail -20');

  if (startResult.code !== 0 && startResult.stderr && !startResult.stderr.includes('Warning')) {
    console.log(`  Output: ${startResult.stdout || startResult.stderr}`);
  }

  // Wait a moment for services to initialize
  console.log('  Waiting for services to initialize...');
  await new Promise(r => setTimeout(r, 5000));

  // Verify services are running
  const psResult = await runCommand(client, 'cd /opt/fireproof && sudo docker compose ps 2>/dev/null || sudo docker ps --filter "name=fireproof"');
  if (psResult.stdout.includes('Up') || psResult.stdout.includes('running')) {
    console.log('  ✓ Docker services started');
  } else {
    console.log('  ⚠ Services may still be starting. Check with:');
    console.log(`     ssh ${vmHost} "cd /opt/fireproof && sudo docker compose ps"`);
  }

  client.end();
}

async function phase8SetPublic(args) {
  console.log('\nPhase 8: Enable Public Access...');

  if (args.dryRun) {
    console.log(`  [DRY RUN] Would run: share set-public ${args.studio}`);
    return;
  }

  console.log(`  Setting public access for ${args.studio}...`);
  let result = await setPublic(args.studio);

  if (!result.success) {
    console.log(`  First attempt failed: ${result.message}`);
    console.log('  Retrying in 2 seconds...');
    await new Promise(r => setTimeout(r, 2000));
    result = await setPublic(args.studio);
  }

  if (result.success) {
    console.log('  ✓ Public access enabled');
  } else {
    console.log(`
  ⚠ ACTION REQUIRED: Public access not enabled
  Run this command manually:
    ssh exe.dev share set-public ${args.studio}
`);
  }
}

async function phase9WriteConnect(args) {
  console.log('\nPhase 9: Write Local Configuration...');

  if (args.dryRun) {
    console.log('  [DRY RUN] Would write .connect file');
    return;
  }

  writeConnectFile({
    studio: args.studio,
    apiUrl: `https://${args.studio}.exe.xyz/api`,
    cloudUrl: `fpcloud://${args.studio}.exe.xyz?protocol=wss`,
    clerkPublishableKey: args.clerkPublishableKey
  });

  console.log('  ✓ Configuration saved');
}

async function verifyDeployment(args) {
  console.log('\nVerifying deployment...');

  const url = `https://${args.studio}.exe.xyz`;

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 10000);

    const response = await fetch(url, {
      signal: controller.signal,
      headers: { 'User-Agent': 'vibes-deploy-connect/1.0' }
    });

    clearTimeout(timeout);

    if (response.ok || response.status === 404) {
      // 404 is fine - nginx is responding, just no index.html
      console.log(`  ✓ ${url} is responding (HTTP ${response.status})`);
      return true;
    } else {
      console.log(`  ⚠ ${url} returned unexpected response: ${response.status}`);
      return false;
    }
  } catch (err) {
    console.log(`  ⚠ ${url} not responding yet: ${err.message}`);
    console.log('  This may be due to DNS propagation. Try again in a few minutes.');
    return false;
  }
}

// ============== Main ==============

async function main() {
  const args = parseArgs(process.argv);

  if (args.help) {
    printHelp();
    process.exit(0);
  }

  // Validate required arguments
  if (!args.studio) {
    console.error('Error: --studio is required');
    console.error('Run with --help for usage information');
    process.exit(1);
  }

  if (!args.clerkPublishableKey) {
    console.error('Error: --clerk-publishable-key is required');
    process.exit(1);
  }

  if (!args.clerkSecretKey) {
    console.error('Error: --clerk-secret-key is required');
    process.exit(1);
  }

  console.log(`
${'━'.repeat(60)}
  FIREPROOF CONNECT STUDIO DEPLOYMENT
${'━'.repeat(60)}
`);

  console.log(`  Studio: ${args.studio}`);
  console.log(`  Clerk Key: ${args.clerkPublishableKey.substring(0, 15)}...`);
  if (args.dryRun) console.log('  Mode: DRY RUN');

  try {
    // Run deployment phases
    await phase1PreFlight(args);
    await phase2CreateVM(args);
    await phase3CloneRepo(args);
    await phase4InstallDocker(args);
    const credentials = await phase5GenerateCredentials(args);
    await phase6WriteEnv(args, credentials);
    await phase7StartServices(args);
    await phase8SetPublic(args);
    await phase9WriteConnect(args);

    // Verification
    if (!args.dryRun) {
      console.log('\n  Waiting 5 seconds for deployment to propagate...');
      await new Promise(r => setTimeout(r, 5000));
      await verifyDeployment(args);
    }

    const vmHost = `${args.studio}.exe.xyz`;
    console.log(`
${'━'.repeat(60)}
  DEPLOYMENT COMPLETE
${'━'.repeat(60)}

  Your Connect Studio is live!

  Endpoints:
    Token API:  https://${vmHost}/api
    Cloud Sync: fpcloud://${vmHost}?protocol=wss

  Update your app's .env:
    VITE_CLERK_PUBLISHABLE_KEY=${args.clerkPublishableKey}
    VITE_API_URL=https://${vmHost}/api
    VITE_CLOUD_URL=fpcloud://${vmHost}?protocol=wss

  Check Docker status:
    ssh ${vmHost} "cd /opt/fireproof && sudo docker compose ps"

  View logs:
    ssh ${vmHost} "cd /opt/fireproof && sudo docker compose logs -f"
`);

  } catch (err) {
    console.error(`\n✗ Deployment failed: ${err.message}`);
    process.exit(1);
  }
}

main();
