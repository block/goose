#!/usr/bin/env node
/**
 * Vite Assembly Script for Connect-enabled Vibes Apps
 *
 * Takes user's JSX/TSX code and assembles it into a Vite project,
 * then builds it for deployment.
 *
 * Usage:
 *   node scripts/assemble-vite.js <input.jsx> <output-dir> [options]
 *
 * Options:
 *   --api-url <url>    Token API URL (default: from .env or localhost)
 *   --cloud-url <url>  Cloud sync URL (default: from .env or localhost)
 *   --clerk-key <key>  Clerk publishable key
 *   --db-name <name>   Database name (default: vibes-app)
 *   --skip-build       Skip npm build step (for testing)
 *
 * Example:
 *   node scripts/assemble-vite.js app.jsx ./build \
 *     --clerk-key pk_test_xxx \
 *     --api-url https://vibes-connect.exe.xyz/api \
 *     --cloud-url fpcloud://vibes-connect.exe.xyz/backend?protocol=wss
 */

import { existsSync, readFileSync, writeFileSync, mkdirSync, cpSync, rmSync } from 'fs';
import { dirname, join, basename } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Find the plugin directory (where templates live)
const findPluginDir = async () => {
  try {
    const { findPluginDirectory } = await import('./find-plugin.js');
    return await findPluginDirectory();
  } catch {
    // Fallback to relative path from scripts/
    return join(__dirname, '..');
  }
};

function parseArgs(argv) {
  const args = {
    input: null,
    outputDir: null,
    apiUrl: null,
    cloudUrl: null,
    clerkKey: null,
    dbName: 'vibes-app',
    skipBuild: false,
    help: false
  };

  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--api-url' && argv[i + 1]) {
      args.apiUrl = argv[++i];
    } else if (arg === '--cloud-url' && argv[i + 1]) {
      args.cloudUrl = argv[++i];
    } else if (arg === '--clerk-key' && argv[i + 1]) {
      args.clerkKey = argv[++i];
    } else if (arg === '--db-name' && argv[i + 1]) {
      args.dbName = argv[++i];
    } else if (arg === '--skip-build') {
      args.skipBuild = true;
    } else if (arg === '--help' || arg === '-h') {
      args.help = true;
    } else if (!args.input) {
      args.input = arg;
    } else if (!args.outputDir) {
      args.outputDir = arg;
    }
  }

  return args;
}

function printHelp() {
  console.log(`
Vite Assembly Script for Connect-enabled Vibes Apps
====================================================

Takes user's JSX/TSX code and assembles it into a Vite project for deployment.

Usage:
  node scripts/assemble-vite.js <input.jsx> <output-dir> [options]

Options:
  --api-url <url>    Token API URL (Fireproof dashboard)
  --cloud-url <url>  Cloud sync URL (fpcloud:// protocol)
  --clerk-key <key>  Clerk publishable key
  --db-name <name>   Database name (default: vibes-app)
  --skip-build       Skip npm build step (for testing)
  --help             Show this help message

Example:
  node scripts/assemble-vite.js app.jsx ./build \\
    --clerk-key pk_test_xxx \\
    --api-url https://vibes-connect.exe.xyz/api \\
    --cloud-url fpcloud://vibes-connect.exe.xyz/backend?protocol=wss
`);
}

/**
 * Transform JSX to TSX for Vite compatibility
 * - Replaces use-fireproof imports with @necrodome/fireproof-clerk
 * - Replaces useFireproof with useFireproofClerk
 * - Preserves the structure
 */
function transformJsxToTsx(jsxCode, dbName) {
  let code = jsxCode;

  // First, replace use-fireproof imports with fireproof-clerk
  code = code.replace(
    /from\s+["']use-fireproof["']/g,
    'from "@necrodome/fireproof-clerk"'
  );

  // Replace useFireproof with useFireproofClerk if not already
  code = code.replace(/\buseFireproof\b(?!Clerk)/g, 'useFireproofClerk');

  // Replace any hardcoded database name with the provided one
  // Look for patterns like useFireproofClerk("something") or useFireproof("something")
  // But only if a specific db name was provided (not default)
  if (dbName !== 'vibes-app') {
    code = code.replace(
      /useFireproofClerk\s*\(\s*["']([^"']+)["']\s*\)/g,
      `useFireproofClerk("${dbName}")`
    );
  }

  // If no fireproof-clerk import exists, add it
  if (!code.includes('@necrodome/fireproof-clerk')) {
    code = `import { useFireproofClerk, UserButton, useUser } from "@necrodome/fireproof-clerk";\n${code}`;
  }

  return code;
}

/**
 * Extract the main component name from JSX code
 */
function extractComponentName(code) {
  // Look for export default function ComponentName
  const defaultExportMatch = code.match(/export\s+default\s+function\s+(\w+)/);
  if (defaultExportMatch) return defaultExportMatch[1];

  // Look for export function ComponentName (first one)
  const namedExportMatch = code.match(/export\s+function\s+(\w+)/);
  if (namedExportMatch) return namedExportMatch[1];

  // Look for function App or function Main
  const commonMatch = code.match(/function\s+(App|Main|UserApp)\s*\(/);
  if (commonMatch) return commonMatch[1];

  return 'UserApp';
}

async function main() {
  const args = parseArgs(process.argv);

  if (args.help) {
    printHelp();
    process.exit(0);
  }

  if (!args.input || !args.outputDir) {
    console.error('Error: Both input file and output directory are required');
    console.error('Run with --help for usage information');
    process.exit(1);
  }

  // Validate input file exists
  if (!existsSync(args.input)) {
    console.error(`Error: Input file not found: ${args.input}`);
    process.exit(1);
  }

  console.log('\n=== Vite Assembly for Connect Apps ===\n');
  console.log(`  Input: ${args.input}`);
  console.log(`  Output: ${args.outputDir}`);
  if (args.clerkKey) console.log(`  Clerk Key: ${args.clerkKey.substring(0, 15)}...`);
  if (args.apiUrl) console.log(`  API URL: ${args.apiUrl}`);
  if (args.cloudUrl) console.log(`  Cloud URL: ${args.cloudUrl}`);
  console.log(`  DB Name: ${args.dbName}`);
  console.log();

  try {
    // Find template directory
    const pluginDir = await findPluginDir();
    const templateDir = join(pluginDir, 'skills/vibes/templates/vite-connect');

    if (!existsSync(templateDir)) {
      throw new Error(`Template directory not found: ${templateDir}`);
    }

    // Clean and create output directory
    if (existsSync(args.outputDir)) {
      console.log('  Cleaning existing output directory...');
      rmSync(args.outputDir, { recursive: true, force: true });
    }
    mkdirSync(args.outputDir, { recursive: true });

    // Copy template to output directory
    console.log('  Copying Vite template...');
    cpSync(templateDir, args.outputDir, { recursive: true });

    // Read and transform user's JSX code
    console.log('  Transforming user code...');
    const userCode = readFileSync(args.input, 'utf-8');
    const transformedCode = transformJsxToTsx(userCode, args.dbName);
    const componentName = extractComponentName(transformedCode);

    // Write transformed code as UserApp.tsx
    const userAppPath = join(args.outputDir, 'src/UserApp.tsx');

    // If the code has a default export, we need to re-export it as UserApp
    let finalCode = transformedCode;
    if (transformedCode.includes('export default')) {
      // Replace export default with named export
      finalCode = transformedCode.replace(
        /export\s+default\s+function\s+(\w+)/,
        'export function UserApp'
      );
    } else if (componentName !== 'UserApp') {
      // Add an alias export
      finalCode = transformedCode + `\n\nexport { ${componentName} as UserApp };\n`;
    }

    writeFileSync(userAppPath, finalCode);
    console.log(`  ✓ User code written to src/UserApp.tsx`);

    // Create .env file with configuration
    console.log('  Creating .env configuration...');
    const envContent = [
      args.clerkKey ? `VITE_CLERK_PUBLISHABLE_KEY=${args.clerkKey}` : '# VITE_CLERK_PUBLISHABLE_KEY=pk_test_xxx',
      args.apiUrl ? `VITE_API_URL=${args.apiUrl}` : '# VITE_API_URL=http://localhost:8080/api/',
      args.cloudUrl ? `VITE_CLOUD_URL=${args.cloudUrl}` : '# VITE_CLOUD_URL=fpcloud://localhost:8080?protocol=ws'
    ].join('\n') + '\n';

    writeFileSync(join(args.outputDir, '.env'), envContent);
    console.log('  ✓ .env created');

    // Install dependencies and build
    if (!args.skipBuild) {
      console.log('  Installing dependencies...');
      execSync('npm install', {
        cwd: args.outputDir,
        stdio: 'pipe',
        timeout: 120000
      });
      console.log('  ✓ Dependencies installed');

      console.log('  Building with Vite...');
      execSync('npm run build', {
        cwd: args.outputDir,
        stdio: 'pipe',
        timeout: 120000
      });
      console.log('  ✓ Build complete');

      console.log(`\n  Output: ${join(args.outputDir, 'dist/')}`);
      console.log('  Deploy this folder to your server.\n');
    } else {
      console.log('\n  Skipped build step (--skip-build)');
      console.log(`  To build manually: cd ${args.outputDir} && npm install && npm run build\n`);
    }

  } catch (err) {
    console.error(`\n✗ Assembly failed: ${err.message}`);
    if (err.stderr) {
      console.error('stderr:', err.stderr.toString());
    }
    process.exit(1);
  }
}

main();
