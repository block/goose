#!/usr/bin/env node
/**
 * One-off Vite UI probe: open localhost:5174, detect chat-input / onboarding / dialog,
 * attempt to type, screenshot + JSON result.
 * Findings-only harness — does not modify product code.
 */
import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(
  '/Users/genarionogueira/Documents/avcd/avcd-agent/ui/node_modules/playwright'
);

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const artifactsDir = join(root, 'artifacts');
const findingsDir = join(root, 'findings');
mkdirSync(artifactsDir, { recursive: true });
mkdirSync(findingsDir, { recursive: true });

const screenshotPath = join(artifactsDir, 'ui-screenshot.png');
const probeMetaPath = join(artifactsDir, 'ui-type-probe-meta.json');
const findingsPath = join(findingsDir, 'ui.json');

const TARGET = 'http://localhost:5174/';
const TYPE_TEXT = 'ui-type-probe-hello';

const result = {
  ts: new Date().toISOString(),
  target: TARGET,
  harness: 'playwright-chromium-against-vite',
  spa_title: null,
  gates: {
    chat_input: false,
    onboarding_welcome: false,
    telemetry_dialog: false,
    provider_error: false,
    role_dialog: false,
    role_textbox: false,
  },
  visible_text_samples: [],
  typing: {
    attempted: false,
    target: null,
    succeeded: false,
    value_after: null,
    error: null,
  },
  screenshot: screenshotPath,
  notes: [],
};

function pushFinding(findings, finding) {
  findings.push(finding);
}

async function main() {
  const findings = [];
  let browser;
  try {
    browser = await chromium.launch({
      headless: true,
      channel: 'chrome', // system Chrome — avoids mismatched ms-playwright browser revision
    });
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const resp = await page.goto(TARGET, { waitUntil: 'networkidle', timeout: 30000 });
    result.http_status = resp?.status() ?? null;
    result.spa_title = await page.title();

    // Wait briefly for React gates
    await page.waitForTimeout(2500);

    const chat = page.locator('[data-testid="chat-input"]');
    const dialog = page.locator('[role="dialog"]');
    const textbox = page.locator('[role="textbox"], textarea, [contenteditable="true"]');

    result.gates.chat_input = (await chat.count()) > 0 && (await chat.first().isVisible().catch(() => false));
    result.gates.role_dialog = (await dialog.count()) > 0 && (await dialog.first().isVisible().catch(() => false));
    result.gates.role_textbox = (await textbox.count()) > 0;

    const bodyText = (await page.locator('body').innerText().catch(() => '')) || '';
    result.visible_text_samples = bodyText
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean)
      .slice(0, 40);

    const lower = bodyText.toLowerCase();
    result.gates.onboarding_welcome =
      lower.includes('welcome to goose') ||
      lower.includes('connect an ai model provider') ||
      lower.includes('welcome to avocado');
    result.gates.telemetry_dialog =
      result.gates.role_dialog &&
      (lower.includes('help improve goose') ||
        lower.includes('anonymous usage data') ||
        lower.includes('share anonymous'));
    result.gates.provider_error =
      lower.includes('unable to connect to avocado work server') ||
      lower.includes('server may be starting');

    // Attempt type into best available target
    let typeTarget = null;
    if (result.gates.chat_input) {
      typeTarget = chat.first();
      result.typing.target = 'chat-input';
    } else if ((await textbox.count()) > 0) {
      typeTarget = textbox.first();
      result.typing.target = 'generic-textbox';
    }

    if (typeTarget) {
      result.typing.attempted = true;
      try {
        await typeTarget.click({ timeout: 5000 });
        await typeTarget.fill('');
        await typeTarget.type(TYPE_TEXT, { delay: 20 });
        const value =
          (await typeTarget.inputValue().catch(() => null)) ??
          (await typeTarget.evaluate((el) => el.textContent || el.innerText || '').catch(() => null));
        result.typing.value_after = value;
        result.typing.succeeded = typeof value === 'string' && value.includes(TYPE_TEXT);
      } catch (err) {
        result.typing.error = String(err?.message || err);
        result.typing.succeeded = false;
      }
    } else {
      result.notes.push('No chat-input or textbox found; typing not attempted');
    }

    await page.screenshot({ path: screenshotPath, fullPage: true });

    // Build findings
    if (!result.gates.chat_input) {
      pushFinding(findings, {
        id: 'UI-001',
        severity: 'high',
        title: 'chat-input missing in Vite SPA probe',
        summary:
          'Browser probe of http://localhost:5174 did not find visible [data-testid="chat-input"]. User typing into chat cannot work if this gate is absent.',
        observed: {
          gates: result.gates,
          spa_title: result.spa_title,
          visible_text_samples: result.visible_text_samples.slice(0, 15),
        },
        likely_gate: result.gates.onboarding_welcome
          ? 'OnboardingGuard welcome / provider selector'
          : result.gates.telemetry_dialog
            ? 'TelemetryConsentPrompt dialog'
            : result.gates.provider_error
              ? 'OnboardingGuard provider check error'
              : result.gates.role_dialog
                ? 'unknown modal dialog'
                : 'unknown / blank / loading',
        repro_steps: [
          'Ensure Avocado Work Electron + Vite are running (port 5174).',
          'Open Chromium to http://localhost:5174/',
          'Wait for SPA hydrate (~2s).',
          'Query document for [data-testid="chat-input"].',
          'Observe: element missing or not visible; screenshot at stress/artifacts/ui-screenshot.png.',
        ],
        harness_note:
          'Browser-only probe may miss Electron preload/IPC; still valid for Vite UI gates.',
        evidence: {
          screenshot: screenshotPath,
          probe_meta: probeMetaPath,
        },
      });
    }

    if (result.gates.onboarding_welcome) {
      pushFinding(findings, {
        id: 'UI-002',
        severity: 'high',
        title: 'Onboarding welcome UI visible instead of chat',
        summary:
          'Onboarding copy ("Welcome to goose" / connect provider) is visible. OnboardingGuard replaces children until a provider is configured via acpReadDefaults.',
        repro_steps: [
          'Launch Avocado Work with userData Avocado Work.',
          'Load UI at http://localhost:5174/.',
          'Observe full-screen onboarding instead of chat composer.',
          'Attempt to type — no chat-input present.',
        ],
        code_ref: 'ui/desktop/src/components/onboarding/OnboardingGuard.tsx',
        evidence: { screenshot: screenshotPath },
      });
    }

    if (result.gates.telemetry_dialog) {
      pushFinding(findings, {
        id: 'UI-003',
        severity: 'medium',
        title: 'Telemetry consent dialog open',
        summary:
          'TelemetryConsentPrompt Dialog is open (modal). Opens when TELEMETRY_UI_ENABLED, GOOSE_PROVIDER is set, and GOOSE_TELEMETRY_ENABLED is null. Modal can intercept focus/keys.',
        repro_steps: [
          'Ensure GOOSE_PROVIDER is set and GOOSE_TELEMETRY_ENABLED is unset/null.',
          'Load app; Dialog "Help improve goose" appears with open=true.',
          'Try typing into underlying UI — focus stays on dialog buttons.',
        ],
        code_ref: 'ui/desktop/src/components/TelemetryConsentPrompt.tsx',
        evidence: { screenshot: screenshotPath },
      });
    }

    if (result.gates.provider_error) {
      pushFinding(findings, {
        id: 'UI-004',
        severity: 'high',
        title: 'Provider check error screen blocking chat',
        summary:
          'OnboardingGuard shows "Unable to connect to Avocado Work server" after acpReadDefaults failures — chat children not rendered.',
        repro_steps: [
          'Load UI while ACP/config backend fails.',
          'See error screen with Retry.',
          'Confirm chat-input absent.',
        ],
        code_ref: 'ui/desktop/src/components/onboarding/OnboardingGuard.tsx',
        evidence: { screenshot: screenshotPath },
      });
    }

    if (result.typing.attempted && !result.typing.succeeded) {
      pushFinding(findings, {
        id: 'UI-005',
        severity: 'high',
        title: 'Typing into available input failed',
        summary: `Attempted type into ${result.typing.target} but value did not contain probe string.`,
        observed: result.typing,
        repro_steps: [
          'Open http://localhost:5174/',
          `Click ${result.typing.target} and type "${TYPE_TEXT}".`,
          'Observe value does not update as expected.',
        ],
        evidence: { screenshot: screenshotPath },
      });
    }

    if (result.gates.chat_input && result.typing.succeeded) {
      pushFinding(findings, {
        id: 'UI-OK-001',
        severity: 'info',
        title: 'Browser probe: typing works on chat-input',
        summary:
          'Vite SPA shows chat-input and accepts typed text in Chromium. If user still cannot type in Electron, cause is likely Electron-specific (focus, preload, OS key routing), not Vite gate.',
        observed: result.typing,
        repro_steps: [
          'Open http://localhost:5174/ in Chromium.',
          'Focus [data-testid="chat-input"] and type.',
          'Confirm characters appear.',
        ],
        evidence: { screenshot: screenshotPath },
      });
    }

    if (findings.length === 0) {
      pushFinding(findings, {
        id: 'UI-HARNESS-001',
        severity: 'info',
        title: 'Probe completed with no classified gate',
        summary: 'SPA loaded but no chat-input/onboarding/telemetry pattern matched confidently.',
        observed: result,
        repro_steps: ['Re-run stress/scripts/ui-type-probe.mjs', 'Inspect screenshot'],
        harness_failure: false,
      });
    }

    writeFileSync(probeMetaPath, JSON.stringify(result, null, 2));
    writeFileSync(findingsPath, JSON.stringify(findings, null, 2));
    console.log(JSON.stringify({ ok: true, findings_count: findings.length, typing: result.typing, gates: result.gates }, null, 2));
  } catch (err) {
    const harnessFindings = [
      {
        id: 'UI-HARNESS-FAIL',
        severity: 'high',
        title: 'UI type probe harness failure',
        summary: String(err?.message || err),
        harness_failure: true,
        repro_steps: [
          'cd avcd-agent',
          'node stress/scripts/ui-type-probe.mjs',
          'Ensure Vite serves http://localhost:5174 and Playwright browsers are installed',
        ],
      },
    ];
    writeFileSync(findingsPath, JSON.stringify(harnessFindings, null, 2));
    writeFileSync(probeMetaPath, JSON.stringify({ ...result, fatal: String(err?.stack || err) }, null, 2));
    console.error(err);
    process.exitCode = 1;
  } finally {
    if (browser) await browser.close();
  }
}

main();
