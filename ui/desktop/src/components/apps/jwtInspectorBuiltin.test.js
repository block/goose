/**
 * @vitest-environment jsdom
 */
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

const require = createRequire(import.meta.url);
const { JSDOM } = require('jsdom');
const JWT_INSPECTOR_HTML_PATH = path.resolve(
  process.cwd(),
  '../../crates/goose/src/goose_apps/jwt-inspector.html'
);

function createJwtInspectorDom({ clipboardWriteText } = {}) {
  const html = fs.readFileSync(JWT_INSPECTOR_HTML_PATH, 'utf8');
  const dom = new JSDOM(html, {
    runScripts: 'dangerously',
    url: 'https://security-goose.local/ui://apps/jwt-inspector',
  });

  Object.defineProperty(dom.window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: clipboardWriteText ?? vi.fn().mockResolvedValue(undefined),
    },
  });

  return dom;
}

describe('JWT Inspector built-in app', () => {
  it('uses the flat Goose-style layout instead of legacy cards and panels', () => {
    const html = fs.readFileSync(JWT_INSPECTOR_HTML_PATH, 'utf8');

    expect(html).not.toContain('class="panel"');
    expect(html).not.toContain('class="card"');
    expect(html).not.toContain('class="block"');
    expect(html).not.toContain('class="cards"');
    expect(html).toContain('class="workspace"');
    expect(html).toContain('class="pane"');
    expect(html).toContain('class="result-section"');
    expect(html).toContain('class="result-row"');
  });

  it('renders a Chinese-first token review interface', () => {
    const dom = createJwtInspectorDom();
    const { document } = dom.window;

    expect(document.querySelector('html')?.lang).toBe('zh-CN');
    expect(document.title).toContain('JWT');
    expect(document.querySelector('h1')?.textContent).toContain('JWT');
    expect(document.body.textContent).toContain('Token 输入');
    expect(document.getElementById('inspectBtn')?.textContent).toContain('分析 Token');
    expect(document.getElementById('loadDemoBtn')?.textContent).toContain('载入演示样例');
    expect(document.body.textContent).toContain('时间字段');
    expect(document.body.textContent).toContain('风险提示');
    expect(document.body.textContent).toContain('签名状态');

    dom.window.close();
  });

  it('flags unsigned and expired tokens while rendering structured claims', () => {
    const dom = createJwtInspectorDom();
    const { document } = dom.window;
    const token =
      'eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJpc3MiOiJodHRwczovL2FwaS5leGFtcGxlLmNuIiwic3ViIjoiYW5hbHlzdC0xIiwiYXVkIjoiZ29vc2UtY2xpZW50IiwiaWF0IjoxNjAwMDAwMDAwLCJuYmYiOjE2MDAwMDAwMDAsImV4cCI6MTYwMDAwMzYwMH0.';

    document.getElementById('tokenInput').value = token;
    document.getElementById('inspectBtn').click();

    const summaryLabels = Array.from(document.querySelectorAll('.summary-item .label')).map((node) =>
      node.textContent?.trim()
    );
    const bodyText = document.body.textContent ?? '';

    expect(summaryLabels).toEqual(
      expect.arrayContaining(['算法', '主题', '签发方', '签名长度'])
    );
    expect(bodyText).toContain('"iss": "https://api.example.cn"');
    expect(bodyText).toContain('"sub": "analyst-1"');
    expect(bodyText).toContain('alg=none');
    expect(bodyText).toContain('exp');
    expect(bodyText).toContain('已过期');
    expect(bodyText).toContain('时间字段');
    expect(bodyText).toContain('签名状态');
    expect(bodyText).toContain('未签名');
    expect(document.querySelector('.result-section')).not.toBeNull();
    expect(document.querySelector('.result-row')).not.toBeNull();
    expect(document.querySelector('.card')).toBeNull();
    expect(document.querySelector('.panel')).toBeNull();
    expect(document.querySelector('[data-copy="signature"]')).not.toBeNull();

    dom.window.close();
  });

  it('copies structured report with clipboard fallback', async () => {
    const clipboardWriteText = vi.fn().mockRejectedValue(new Error('clipboard denied'));
    const dom = createJwtInspectorDom({ clipboardWriteText });
    const { document } = dom.window;
    document.execCommand = vi.fn().mockReturnValue(true);

    document.getElementById('tokenInput').value =
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTEiLCJpc3MiOiJzZWN1cml0eS1nb29zZSIsImV4cCI6MTkxODU4MDAwMH0.signature';
    document.getElementById('inspectBtn').click();

    const copyButton = document.querySelector('[data-copy="report"]');
    copyButton?.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(clipboardWriteText).toHaveBeenCalled();
    expect(document.execCommand).toHaveBeenCalledWith('copy');
    expect(document.getElementById('status')?.textContent).toContain('已复制');

    dom.window.close();
  });

  it('copies the signature segment separately', async () => {
    const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
    const dom = createJwtInspectorDom({ clipboardWriteText });
    const { document } = dom.window;

    document.getElementById('tokenInput').value =
      'Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTEiLCJpc3MiOiJzZWN1cml0eS1nb29zZSIsImV4cCI6MTkxODU4MDAwMH0.signature-segment-value';
    document.getElementById('inspectBtn').click();

    const copyButton = document.querySelector('[data-copy="signature"]');
    copyButton?.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(clipboardWriteText).toHaveBeenCalledWith('signature-segment-value');
    expect(document.getElementById('status')?.textContent).toContain('已复制签名段');

    dom.window.close();
  });
});
