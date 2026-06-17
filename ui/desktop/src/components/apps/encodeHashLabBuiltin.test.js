/**
 * @vitest-environment jsdom
 */
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

const require = createRequire(import.meta.url);
const { JSDOM } = require('jsdom');
const { webcrypto } = require('node:crypto');
const ENCODE_HASH_LAB_HTML_PATH = path.resolve(
  process.cwd(),
  '../../crates/goose/src/goose_apps/encode-hash-lab.html'
);

function createEncodeHashLabDom({ clipboardWriteText } = {}) {
  const html = fs.readFileSync(ENCODE_HASH_LAB_HTML_PATH, 'utf8');
  const dom = new JSDOM(html, {
    runScripts: 'dangerously',
    url: 'https://security-goose.local/ui://apps/encode-hash-lab',
  });

  Object.defineProperty(dom.window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: clipboardWriteText ?? vi.fn().mockResolvedValue(undefined),
    },
  });
  Object.defineProperty(dom.window, 'crypto', {
    configurable: true,
    value: webcrypto,
  });

  return dom;
}

function addOperation(document, value) {
  const select = document.getElementById('operationSelect');
  const addButton = document.getElementById('addStepBtn');
  select.value = value;
  addButton.click();
}

describe('Encode & Hash Lab built-in app', () => {
  it('renders a Chinese-first pipeline interface', () => {
    const dom = createEncodeHashLabDom();
    const { document } = dom.window;
    const html = fs.readFileSync(ENCODE_HASH_LAB_HTML_PATH, 'utf8');
    const styleText = document.querySelector('style')?.textContent ?? '';
    const stepResultRule = styleText.match(/\.step-result\s*\{[^}]+\}/)?.[0] ?? '';

    expect(document.querySelector('html')?.lang).toBe('zh-CN');
    expect(document.title).toContain('编码');
    expect(document.querySelector('h1')?.textContent).toContain('编码');
    expect(document.body.textContent).toContain('操作链');
    expect(document.body.textContent).toContain('最终输出');
    expect(document.body.textContent).toContain('编码 / 解码');
    expect(document.body.textContent).toContain('哈希');
    expect(document.body.textContent).toContain('格式整理');
    expect(document.body.textContent).toContain('安全辅助');
    expect(document.getElementById('addStepBtn')?.textContent).toContain('加入步骤');
    expect(document.getElementById('runPipelineBtn')?.textContent).toContain('执行操作链');
    expect(document.body.textContent).not.toContain('当前已支持');
    expect(document.body.textContent).not.toContain('当前未覆盖');
    expect(stepResultRule).toContain('max-height');
    expect(stepResultRule).toContain('overflow: auto;');
    expect(html).toContain('class="workspace"');
    expect(html).toContain('class="pane"');
    expect(html).not.toContain('class="panel"');
    expect(html).not.toContain('matrix-card');

    dom.window.close();
  });

  it('executes a multi-step pipeline and shows intermediate outputs', async () => {
    const dom = createEncodeHashLabDom();
    const { document } = dom.window;
    const input = document.getElementById('plainInput');
    const runButton = document.getElementById('runPipelineBtn');

    input.value = 'hello security goose';
    addOperation(document, 'base64-encode');
    addOperation(document, 'url-encode');
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    const finalOutput = document.getElementById('finalOutput')?.value ?? '';
    const stepTitles = Array.from(document.querySelectorAll('[data-step-title]')).map((node) =>
      node.textContent?.trim()
    );
    const stepResults = Array.from(document.querySelectorAll('[data-step-result]')).map((node) =>
      node.textContent?.trim()
    );

    expect(stepTitles).toEqual(['Base64 编码', 'URL 编码']);
    expect(stepResults[0]).toBe('aGVsbG8gc2VjdXJpdHkgZ29vc2U=');
    expect(stepResults[1]).toContain('%3D');
    expect(finalOutput).toBe(stepResults[1]);

    dom.window.close();
  });

  it('supports reordering and removing pipeline steps before execution', async () => {
    const dom = createEncodeHashLabDom();
    const { document } = dom.window;
    const input = document.getElementById('plainInput');
    const runButton = document.getElementById('runPipelineBtn');

    input.value = 'Njg2OQ==';
    addOperation(document, 'hex-decode');
    addOperation(document, 'base64-decode');

    document.querySelector('[data-step-move-up="1"]')?.dispatchEvent(
      new dom.window.MouseEvent('click', { bubbles: true })
    );
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(document.getElementById('finalOutput')?.value).toBe('hi');

    document.querySelector('[data-step-remove="0"]')?.dispatchEvent(
      new dom.window.MouseEvent('click', { bubbles: true })
    );
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(document.getElementById('status')?.textContent).toContain('失败');

    dom.window.close();
  });

  it('supports high-frequency operations for json, jwt, hashes, and unicode transforms', async () => {
    const dom = createEncodeHashLabDom();
    const { document } = dom.window;
    const input = document.getElementById('plainInput');
    const runButton = document.getElementById('runPipelineBtn');

    input.value = '{"a":1,"b":[2,3]}';
    addOperation(document, 'json-pretty');
    addOperation(document, 'unicode-escape-encode');
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    expect(document.getElementById('finalOutput')?.value).toContain('\\u');

    document.getElementById('clearBtn')?.click();
    input.value =
      'eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ1c2VyLTEiLCJhZG1pbiI6dHJ1ZX0.';
    addOperation(document, 'jwt-decode');
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    expect(document.getElementById('finalOutput')?.value).toContain('"sub": "user-1"');
    expect(document.getElementById('finalOutput')?.value).toContain('"alg": "none"');

    document.getElementById('clearBtn')?.click();
    input.value = 'abc';
    addOperation(document, 'md5-hash');
    addOperation(document, 'sha256-hash');
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    const stepResults = Array.from(document.querySelectorAll('[data-step-result]')).map((node) =>
      node.textContent?.trim()
    );
    expect(stepResults[0]).toBe('900150983cd24fb0d6963f7d28e17f72');
    expect(stepResults[1]).toBe(
      '2c89b7e560fb8c30d1c61408e91e4a84934ff0d24e68e51a6fdb744a1bb717fe'
    );

    dom.window.close();
  });

  it('copies final output and falls back when clipboard api is unavailable', async () => {
    const clipboardWriteText = vi.fn().mockRejectedValue(new Error('clipboard denied'));
    const dom = createEncodeHashLabDom({ clipboardWriteText });
    const { document } = dom.window;
    document.execCommand = vi.fn().mockReturnValue(true);

    const input = document.getElementById('plainInput');
    const runButton = document.getElementById('runPipelineBtn');
    input.value = 'abc';
    addOperation(document, 'base64-encode');
    runButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    document.getElementById('copyFinalBtn')?.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(clipboardWriteText).toHaveBeenCalled();
    expect(document.execCommand).toHaveBeenCalledWith('copy');
    expect(document.getElementById('status')?.textContent).toContain('已复制');

    dom.window.close();
  });
});
