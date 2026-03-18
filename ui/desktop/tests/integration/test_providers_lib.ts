/**
 * Shared library for provider smoke tests.
 *
 * Ported from scripts/test_providers_lib.sh — keeps the same provider config,
 * allowed-failure list, agentic-provider list, and environment detection.
 */

import { execSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// ---------------------------------------------------------------------------
// Provider configuration
// ---------------------------------------------------------------------------

const PROVIDER_CONFIG_RAW = `
openrouter -> google/gemini-2.5-pro|anthropic/claude-sonnet-4.5|qwen/qwen3-coder:exacto|z-ai/glm-4.6:exacto|nvidia/nemotron-3-nano-30b-a3b
xai -> grok-3
openai -> gpt-4o|gpt-4o-mini|gpt-3.5-turbo|gpt-5
anthropic -> claude-sonnet-4-5-20250929|claude-opus-4-5-20251101
google -> gemini-2.5-pro|gemini-2.5-flash|gemini-3-pro-preview|gemini-3-flash-preview
tetrate -> claude-sonnet-4-20250514
databricks -> databricks-claude-sonnet-4|gemini-2-5-flash|gpt-4o
azure_openai -> \${AZURE_OPENAI_DEPLOYMENT_NAME}
aws_bedrock -> us.anthropic.claude-sonnet-4-5-20250929-v1:0
gcp_vertex_ai -> gemini-2.5-pro
snowflake -> claude-sonnet-4-5
venice -> llama-3.3-70b
litellm -> gpt-4o-mini
sagemaker_tgi -> sagemaker-tgi-endpoint
github_copilot -> gpt-4.1
chatgpt_codex -> gpt-5.1-codex
claude-code -> default
codex -> gpt-5.2-codex
gemini-cli -> gemini-2.5-pro
cursor-agent -> auto
ollama -> qwen3
`;

const ALLOWED_FAILURES = new Set([
  'google:gemini-2.5-flash',
  'google:gemini-3-pro-preview',
  'openrouter:nvidia/nemotron-3-nano-30b-a3b',
  'openrouter:qwen/qwen3-coder:exacto',
  'openai:gpt-3.5-turbo',
]);

const AGENTIC_PROVIDERS = new Set(['claude-code', 'codex', 'gemini-cli', 'cursor-agent']);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function loadDotenv(): void {
  const envPath = path.resolve(process.cwd(), '.env');
  if (!fs.existsSync(envPath)) return;
  const lines = fs.readFileSync(envPath, 'utf-8').split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx);
    const value = trimmed.slice(eqIdx + 1);
    if (!(key in process.env)) {
      process.env[key] = value;
    }
  }
}

function hasEnv(name: string): boolean {
  return !!process.env[name];
}

function hasCmd(name: string): boolean {
  try {
    execSync(`command -v ${name}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

function hasFile(p: string): boolean {
  return fs.existsSync(p);
}

export function isAgenticProvider(provider: string): boolean {
  return AGENTIC_PROVIDERS.has(provider);
}

function isProviderAvailable(provider: string): boolean {
  switch (provider) {
    case 'openrouter':
      return hasEnv('OPENROUTER_API_KEY');
    case 'xai':
      return hasEnv('XAI_API_KEY');
    case 'openai':
      return hasEnv('OPENAI_API_KEY');
    case 'anthropic':
      return hasEnv('ANTHROPIC_API_KEY');
    case 'google':
      return hasEnv('GOOGLE_API_KEY');
    case 'tetrate':
      return hasEnv('TETRATE_API_KEY');
    case 'databricks':
      return hasEnv('DATABRICKS_HOST') && hasEnv('DATABRICKS_TOKEN');
    case 'azure_openai':
      return hasEnv('AZURE_OPENAI_ENDPOINT') && hasEnv('AZURE_OPENAI_DEPLOYMENT_NAME');
    case 'aws_bedrock':
      return hasEnv('AWS_REGION') && (hasEnv('AWS_PROFILE') || hasEnv('AWS_ACCESS_KEY_ID'));
    case 'gcp_vertex_ai':
      return hasEnv('GCP_PROJECT_ID');
    case 'snowflake':
      return hasEnv('SNOWFLAKE_HOST') && hasEnv('SNOWFLAKE_TOKEN');
    case 'venice':
      return hasEnv('VENICE_API_KEY');
    case 'litellm':
      return hasEnv('LITELLM_API_KEY');
    case 'sagemaker_tgi':
      return hasEnv('SAGEMAKER_ENDPOINT_NAME') && hasEnv('AWS_REGION');
    case 'github_copilot':
      return (
        hasEnv('GITHUB_COPILOT_TOKEN') ||
        hasFile(path.join(os.homedir(), '.config/goose/github_copilot_token.json'))
      );
    case 'chatgpt_codex':
      return (
        hasEnv('CHATGPT_CODEX_TOKEN') ||
        hasFile(path.join(os.homedir(), '.config/goose/chatgpt_codex_token.json'))
      );
    case 'ollama':
      return hasEnv('OLLAMA_HOST') || hasCmd('ollama');
    case 'claude-code':
      return hasCmd('claude');
    case 'codex':
      return hasCmd('codex');
    case 'gemini-cli':
      return hasCmd('gemini');
    case 'cursor-agent':
      return hasCmd('cursor-agent');
    default:
      return true;
  }
}

export function isAllowedFailure(provider: string, model: string): boolean {
  return ALLOWED_FAILURES.has(`${provider}:${model}`);
}

function shouldSkipProvider(provider: string): boolean {
  const skip = process.env.SKIP_PROVIDERS;
  if (!skip) return false;
  return skip
    .split(',')
    .map((s) => s.trim())
    .includes(provider);
}

// ---------------------------------------------------------------------------
// Parse provider config
// ---------------------------------------------------------------------------

interface ProviderLine {
  provider: string;
  modelsStr: string;
}

function parseProviderConfig(): ProviderLine[] {
  const lines: ProviderLine[] = [];
  for (const raw of PROVIDER_CONFIG_RAW.split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const arrowIdx = line.indexOf(' -> ');
    if (arrowIdx === -1) continue;
    const provider = line.slice(0, arrowIdx).trim();
    let modelsStr = line.slice(arrowIdx + 4).trim();
    modelsStr = modelsStr.replace(/\$\{(\w+)\}/g, (_, name) => process.env[name] ?? '');
    lines.push({ provider, modelsStr });
  }
  return lines;
}

// ---------------------------------------------------------------------------
// Build goose binary
// ---------------------------------------------------------------------------

export function buildGoose(): string {
  if (!process.env.SKIP_BUILD) {
    console.error('Building goose...');
    execSync('cargo build --bin goose', { stdio: 'inherit' });
    console.error('');
  } else {
    console.error('Skipping build (SKIP_BUILD is set)...');
    console.error('');
  }
  return path.resolve(process.cwd(), '..', '..', 'target/debug/goose');
}

// ---------------------------------------------------------------------------
// Test case discovery
// ---------------------------------------------------------------------------

export interface TestCase {
  provider: string;
  model: string;
  available: boolean;
  skippedReason?: string;
}

export function discoverTestCases(options?: { skipAgentic?: boolean }): TestCase[] {
  loadDotenv();
  const skipAgentic = options?.skipAgentic ?? false;
  const providerLines = parseProviderConfig();

  const testCases: TestCase[] = [];

  for (const { provider, modelsStr } of providerLines) {
    const available = isProviderAvailable(provider);
    const models = modelsStr.split('|');

    for (const model of models) {
      if (!available) {
        testCases.push({
          provider,
          model,
          available: false,
          skippedReason: 'prerequisites not met',
        });
      } else if (shouldSkipProvider(provider)) {
        testCases.push({
          provider,
          model,
          available: false,
          skippedReason: 'SKIP_PROVIDERS',
        });
      } else if (skipAgentic && isAgenticProvider(provider)) {
        testCases.push({
          provider,
          model,
          available: false,
          skippedReason: 'agentic provider skipped in this mode',
        });
      } else {
        testCases.push({ provider, model, available: true });
      }
    }
  }

  return testCases;
}

// ---------------------------------------------------------------------------
// Utility: run goose binary and capture output
// ---------------------------------------------------------------------------

export function runGoose(
  gooseBin: string,
  cwd: string,
  prompt: string,
  builtins: string,
  env: Record<string, string>
): Promise<string> {
  return new Promise((resolve) => {
    const child: ChildProcess = spawn(
      gooseBin,
      ['run', '--text', prompt, '--with-builtin', builtins],
      {
        cwd,
        env: { ...process.env, ...env },
        stdio: ['ignore', 'pipe', 'pipe'],
      }
    );

    let output = '';
    child.stdout?.on('data', (d) => {
      output += String(d);
    });
    child.stderr?.on('data', (d) => {
      output += String(d);
    });

    child.on('close', () => {
      resolve(output);
    });

    child.on('error', (err) => {
      resolve(`spawn error: ${err.message}`);
    });
  });
}
