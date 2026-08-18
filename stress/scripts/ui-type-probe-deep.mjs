#!/usr/bin/env node
import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(
  '/Users/genarionogueira/Documents/avcd/avcd-agent/ui/node_modules/playwright'
);

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
mkdirSync(join(root, 'artifacts'), { recursive: true });

const browser = await chromium.launch({ headless: true, channel: 'chrome' });
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const consoleLogs = [];
const pageErrors = [];
page.on('console', (m) => consoleLogs.push({ type: m.type(), text: m.text() }));
page.on('pageerror', (e) => pageErrors.push(String(e?.message || e)));

await page.goto('http://localhost:5174/', { waitUntil: 'domcontentloaded', timeout: 30000 });
await page.waitForTimeout(8000);

const dump = await page.evaluate(() => {
  const rootEl = document.getElementById('root');
  return {
    title: document.title,
    rootExists: !!rootEl,
    rootHTML: rootEl ? rootEl.innerHTML.slice(0, 5000) : null,
    rootChildCount: rootEl ? rootEl.childElementCount : 0,
    bodyText: (document.body?.innerText || '').slice(0, 2000),
    chatInput: !!document.querySelector('[data-testid="chat-input"]'),
    dialogs: document.querySelectorAll('[role="dialog"]').length,
    textareas: document.querySelectorAll('textarea').length,
    buttons: [...document.querySelectorAll('button')]
      .map((b) => b.textContent?.trim())
      .filter(Boolean)
      .slice(0, 20),
    h1s: [...document.querySelectorAll('h1')].map((h) => h.textContent?.trim()).slice(0, 10),
    hasElectron: typeof window.electron !== 'undefined',
    hasAppConfig: typeof window.appConfig !== 'undefined',
  };
});

await page.screenshot({ path: join(root, 'artifacts', 'ui-screenshot.png'), fullPage: true });
const out = {
  ts: new Date().toISOString(),
  dump,
  pageErrors,
  consoleErrors: consoleLogs.filter((l) => l.type === 'error').slice(0, 40),
  consoleWarnings: consoleLogs.filter((l) => l.type === 'warning').slice(0, 20),
  consoleAllSample: consoleLogs.slice(0, 60),
};
writeFileSync(join(root, 'artifacts', 'ui-type-probe-meta.json'), JSON.stringify(out, null, 2));
console.log(JSON.stringify(out, null, 2));
await browser.close();
