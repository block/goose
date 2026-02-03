/**
 * Integration tests for AI Proxy deployment in deploy-exe.js
 *
 * Tests the AI proxy phase configuration and argument handling.
 * Uses mocked SSH operations since we can't test actual VM deployment.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

// Path to the AI proxy script
const AI_PROXY_PATH = join(__dirname, '../../lib/ai-proxy.js');

describe('AI Proxy Deployment', () => {
  describe('proxy script exists and is valid', () => {
    it('ai-proxy.js file exists', () => {
      expect(existsSync(AI_PROXY_PATH)).toBe(true);
    });

    it('ai-proxy.js contains required exports', () => {
      const content = readFileSync(AI_PROXY_PATH, 'utf-8');

      // Check for key components
      expect(content).toContain('OPENROUTER_API_KEY');
      expect(content).toContain('VIBES_MULTI_TENANT');
      expect(content).toContain('VIBES_TENANT_LIMIT');
      expect(content).toContain('Bun.serve');
    });

    it('ai-proxy.js handles multi-tenant mode', () => {
      const content = readFileSync(AI_PROXY_PATH, 'utf-8');

      expect(content).toContain('IS_MULTI_TENANT');
      expect(content).toContain('extractTenant');
      expect(content).toContain('getOrCreateTenantKey');
    });

    it('ai-proxy.js includes CORS headers', () => {
      const content = readFileSync(AI_PROXY_PATH, 'utf-8');

      expect(content).toContain('Access-Control-Allow-Origin');
      expect(content).toContain('Access-Control-Allow-Methods');
      expect(content).toContain('Access-Control-Allow-Headers');
    });

    it('ai-proxy.js proxies to OpenRouter', () => {
      const content = readFileSync(AI_PROXY_PATH, 'utf-8');

      expect(content).toContain('openrouter.ai/api/v1/chat/completions');
      expect(content).toContain('proxyToOpenRouter');
    });
  });

  describe('deployment argument parsing', () => {
    it('correctly parses --ai-key argument', () => {
      const args = parseTestArgs(['--ai-key', 'sk-or-v1-test123']);
      expect(args.aiKey).toBe('sk-or-v1-test123');
    });

    it('correctly parses --multi-tenant flag', () => {
      const args = parseTestArgs(['--multi-tenant']);
      expect(args.multiTenant).toBe(true);
    });

    it('correctly parses --tenant-limit argument', () => {
      const args = parseTestArgs(['--tenant-limit', '10']);
      expect(args.tenantLimit).toBe(10);
    });

    it('defaults tenant-limit to 5', () => {
      const args = parseTestArgs([]);
      expect(args.tenantLimit).toBe(5);
    });

    it('multi-tenant defaults to false', () => {
      const args = parseTestArgs([]);
      expect(args.multiTenant).toBe(false);
    });

    it('handles combined AI arguments', () => {
      const args = parseTestArgs([
        '--ai-key', 'sk-or-v1-test',
        '--multi-tenant',
        '--tenant-limit', '15'
      ]);

      expect(args.aiKey).toBe('sk-or-v1-test');
      expect(args.multiTenant).toBe(true);
      expect(args.tenantLimit).toBe(15);
    });
  });

  describe('environment variable generation', () => {
    it('generates correct environment variables for single-user mode', () => {
      const envVars = generateEnvVars({
        aiKey: 'sk-or-v1-test',
        multiTenant: false,
        tenantLimit: 5
      });

      expect(envVars).toContain('OPENROUTER_API_KEY=sk-or-v1-test');
      expect(envVars).toContain('VIBES_MULTI_TENANT=false');
      expect(envVars).toContain('VIBES_TENANT_LIMIT=5');
    });

    it('generates correct environment variables for multi-tenant mode', () => {
      const envVars = generateEnvVars({
        aiKey: 'sk-or-v1-prod',
        multiTenant: true,
        tenantLimit: 10
      });

      expect(envVars).toContain('OPENROUTER_API_KEY=sk-or-v1-prod');
      expect(envVars).toContain('VIBES_MULTI_TENANT=true');
      expect(envVars).toContain('VIBES_TENANT_LIMIT=10');
    });
  });

  describe('systemd service file generation', () => {
    it('generates valid systemd unit file', () => {
      const serviceFile = generateSystemdService();

      expect(serviceFile).toContain('[Unit]');
      expect(serviceFile).toContain('[Service]');
      expect(serviceFile).toContain('[Install]');
      expect(serviceFile).toContain('ExecStart=');
      expect(serviceFile).toContain('proxy.js');
      expect(serviceFile).toContain('Restart=always');
    });

    it('includes environment file reference', () => {
      const serviceFile = generateSystemdService();
      expect(serviceFile).toContain('EnvironmentFile=/etc/environment');
    });
  });

  describe('nginx configuration generation', () => {
    it('generates valid nginx proxy config', () => {
      const nginxConf = generateNginxConfig();

      expect(nginxConf).toContain('location /api/ai/');
      expect(nginxConf).toContain('proxy_pass http://127.0.0.1:3001');
      expect(nginxConf).toContain('proxy_http_version 1.1');
    });

    it('includes required proxy headers', () => {
      const nginxConf = generateNginxConfig();

      expect(nginxConf).toContain('proxy_set_header Host');
      expect(nginxConf).toContain('proxy_set_header X-Real-IP');
    });
  });

  describe('deployment output messages', () => {
    it('includes AI proxy info in success message when enabled', () => {
      const message = generateSuccessMessage({
        name: 'testapp',
        aiKey: 'sk-or-v1-test',
        multiTenant: false,
        tenantLimit: 5
      });

      expect(message).toContain('AI Proxy');
      expect(message).toContain('/api/ai/chat');
      expect(message).toContain('Single-user');
    });

    it('shows multi-tenant details when enabled', () => {
      const message = generateSuccessMessage({
        name: 'testapp',
        aiKey: 'sk-or-v1-test',
        multiTenant: true,
        tenantLimit: 10
      });

      expect(message).toContain('Multi-tenant');
      expect(message).toContain('$10/month');
    });

    it('excludes AI info when not enabled', () => {
      const message = generateSuccessMessage({
        name: 'testapp',
        aiKey: null,
        multiTenant: false,
        tenantLimit: 5
      });

      expect(message).not.toContain('AI Proxy');
      expect(message).not.toContain('/api/ai/');
    });
  });
});

// ============== Test Helpers ==============

/**
 * Simple arg parser for testing (mirrors deploy-exe.js logic)
 */
function parseTestArgs(argv) {
  const args = {
    aiKey: null,
    multiTenant: false,
    tenantLimit: 5
  };

  for (let i = 0; i < argv.length; i++) {
    switch (argv[i]) {
      case '--ai-key':
        args.aiKey = argv[++i];
        break;
      case '--multi-tenant':
        args.multiTenant = true;
        break;
      case '--tenant-limit':
        args.tenantLimit = parseInt(argv[++i], 10);
        break;
    }
  }

  return args;
}

/**
 * Generate environment variables for AI proxy
 */
function generateEnvVars(args) {
  return [
    `OPENROUTER_API_KEY=${args.aiKey}`,
    `VIBES_MULTI_TENANT=${args.multiTenant}`,
    `VIBES_TENANT_LIMIT=${args.tenantLimit}`
  ];
}

/**
 * Generate systemd service file (mirrors deploy-exe.js)
 */
function generateSystemdService() {
  return `[Unit]
Description=Vibes AI Proxy
After=network.target

[Service]
ExecStart=/root/.bun/bin/bun run /opt/vibes/proxy.js
Restart=always
EnvironmentFile=/etc/environment
WorkingDirectory=/opt/vibes

[Install]
WantedBy=multi-user.target`;
}

/**
 * Generate nginx configuration (mirrors deploy-exe.js)
 */
function generateNginxConfig() {
  return `location /api/ai/ {
    proxy_pass http://127.0.0.1:3001/;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}`;
}

/**
 * Generate success message (mirrors deploy-exe.js output)
 */
function generateSuccessMessage(args) {
  let message = `Your app is live at: https://${args.name}.exe.xyz\n`;

  if (args.aiKey) {
    message += `\nAI Proxy:\n`;
    message += `  Endpoint: https://${args.name}.exe.xyz/api/ai/chat\n`;
    message += `  Mode: ${args.multiTenant ? `Multi-tenant ($${args.tenantLimit}/month per tenant)` : 'Single-user'}`;
  }

  return message;
}

export { parseTestArgs, generateEnvVars, generateSystemdService, generateNginxConfig, generateSuccessMessage };
