#!/usr/bin/env node
/**
 * Generate HANDOFF.md for exe.dev deployment
 *
 * Creates a context document that gives remote Claude enough
 * information to continue development meaningfully.
 *
 * Usage:
 *   import { generateHandoff } from './generate-handoff.js';
 *
 *   const content = generateHandoff({
 *     appDescription: 'A todo app with real-time sync',
 *     files: ['index.html'],
 *     originalPrompt: 'Build me a todo app',
 *     decisions: '- Used Fireproof for persistence\n- Added dark mode toggle',
 *     nextSteps: 'User mentioned wanting to add categories'
 *   });
 */

/**
 * Generate a context handoff document from session context.
 * This gives remote Claude enough context to continue development.
 *
 * @param {Object} options - Handoff options
 * @param {string} [options.appDescription] - What was built
 * @param {string[]} [options.files] - List of files included
 * @param {string} [options.originalPrompt] - User's original request
 * @param {string} [options.decisions] - Key decisions made during development
 * @param {string} [options.nextSteps] - What to do next
 * @param {string} [options.vmName] - Name of the exe.dev VM
 * @param {boolean} [options.connectEnabled] - Whether Connect (Fireproof + Clerk) is enabled
 * @returns {string} HANDOFF.md content
 */
export function generateHandoff(options = {}) {
  const {
    appDescription = 'A Vibes app',
    files = ['index.html'],
    originalPrompt = 'Build an app',
    decisions = '- Standard Vibes architecture with Fireproof database',
    nextSteps = 'Continue development based on user requests.',
    vmName = 'app',
    connectEnabled = false
  } = options;

  const fileList = Array.isArray(files)
    ? files.map(f => `- \`${f}\``).join('\n')
    : `- \`${files}\``;

  // Docker section for Connect-enabled deployments
  const dockerSection = connectEnabled ? `
## Docker Services (Fireproof Connect)

This deployment includes Fireproof Connect services running in Docker.

### Service Endpoints

| Service | Internal Port | External URL | Purpose |
|---------|--------------|--------------|---------|
| Dashboard (Token API) | 7370 | https://${vmName}.exe.xyz/api | Issues authenticated tokens |
| Cloud Backend (Sync) | 8909 | wss://${vmName}.exe.xyz/backend | Real-time WebSocket sync |

### Docker Commands

\`\`\`bash
# Check service status
cd /opt/fireproof/core && sudo docker compose ps

# View logs (all services)
cd /opt/fireproof/core && sudo docker compose logs -f

# View logs for specific service
cd /opt/fireproof/core && sudo docker compose logs -f dashboard
cd /opt/fireproof/core && sudo docker compose logs -f cloud-backend

# Restart services
cd /opt/fireproof/core && sudo docker compose restart

# Stop services
cd /opt/fireproof/core && sudo docker compose down

# Start services
cd /opt/fireproof/core && sudo docker compose up -d

# Rebuild and restart (after config changes)
cd /opt/fireproof/core && sudo docker compose up -d --build
\`\`\`

### Nginx Proxy Configuration

The Connect services are proxied through nginx:
- \`/api\` → localhost:7370 (Token API)
- \`/backend\` → localhost:8909 (WebSocket sync)

Config file: \`/etc/nginx/vibes-connect.conf\`

### Troubleshooting

\`\`\`bash
# Check if containers are running
sudo docker ps

# Check container health
cd /opt/fireproof/core && sudo docker compose ps --format "table {{.Name}}\\t{{.Status}}\\t{{.Health}}"

# View recent errors
cd /opt/fireproof/core && sudo docker compose logs --tail=50 | grep -i error

# Check nginx proxy status
sudo nginx -t && sudo systemctl status nginx

# Test Token API health
curl -s http://localhost:7370/health

# Test Cloud Backend health
curl -s http://localhost:8909/health
\`\`\`

### Environment Variables

The Connect services use credentials stored in \`/opt/fireproof/core/docker-compose.yaml\`.
Do NOT manually edit this file - regenerate it by rerunning the deploy script.
` : '';

  const handoff = `# Development Handoff

This document was generated during deployment to exe.dev. It provides context
for continuing development on this VM.

## What Was Built

${appDescription}

## Files Included

${fileList}

## User's Original Request

> ${originalPrompt}

## Key Decisions Made

${decisions}

## What To Do Next

${nextSteps}

## Technical Context

**Stack:**
- Framework: React (via Babel transpilation, no build step)
- Database: Fireproof (local-first${connectEnabled ? ', syncs via Connect services' : ''})
- Styling: Tailwind CSS via CDN
- Runtime: Runs directly in browser${connectEnabled ? '\n- Auth: Clerk (via @fireproof/clerk)' : ''}

**Architecture:**
- Single \`index.html\` file with embedded JSX
- \`<script type="text/babel">\` for React components
- Import map for CDN dependencies (esm.sh)${connectEnabled ? '\n- Docker containers for Fireproof Connect services' : '\n- No server-side logic - pure static hosting'}

**Common Commands:**
\`\`\`bash
# View the app
open https://${vmName}.exe.xyz

# Edit the app
nano /var/www/html/index.html

# Check nginx status
sudo systemctl status nginx

# View nginx logs
sudo tail -f /var/log/nginx/access.log
\`\`\`
${dockerSection}
---
*Generated by vibes-skill on ${new Date().toISOString().split('T')[0]}*
`;

  return handoff;
}

/**
 * Extract context from environment variables set by the vibes skill.
 * Falls back to sensible defaults if not available.
 *
 * @returns {Object} Context object for generateHandoff
 */
export function extractContextFromEnv() {
  return {
    appDescription: process.env.VIBES_APP_DESCRIPTION || 'A Vibes app',
    originalPrompt: process.env.VIBES_ORIGINAL_PROMPT || 'Build an app',
    decisions: process.env.VIBES_DECISIONS || '- Standard Vibes architecture with Fireproof database',
    nextSteps: process.env.VIBES_NEXT_STEPS || 'Continue development based on user requests.',
    vmName: process.env.VIBES_VM_NAME || 'app'
  };
}

// CLI usage: node generate-handoff.js [--output path]
if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2);
  const outputIndex = args.indexOf('--output');
  const outputPath = outputIndex !== -1 ? args[outputIndex + 1] : null;

  const context = extractContextFromEnv();
  context.files = ['index.html']; // Default for CLI usage

  const content = generateHandoff(context);

  if (outputPath) {
    const { writeFileSync } = await import('fs');
    writeFileSync(outputPath, content);
    console.log(`HANDOFF.md written to ${outputPath}`);
  } else {
    console.log(content);
  }
}
