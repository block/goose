/**
 * @vitest-environment jsdom
 */
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

const require = createRequire(import.meta.url);
const { JSDOM } = require('jsdom');
const IOC_TOOLBOX_HTML_PATH = path.resolve(process.cwd(), '../../crates/goose/src/goose_apps/ioc-toolbox.html');

function createIocToolboxDom({ clipboardWriteText } = {}) {
  const html = fs.readFileSync(IOC_TOOLBOX_HTML_PATH, 'utf8');
  const dom = new JSDOM(html, {
    runScripts: 'dangerously',
    url: 'https://security-goose.local/ui://apps/ioc-toolbox',
  });

  Object.defineProperty(dom.window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: clipboardWriteText ?? vi.fn().mockResolvedValue(undefined),
    },
  });

  return dom;
}

describe('IOC Toolbox built-in app', () => {
  it('renders a Chinese-first analyst interface', () => {
    const dom = createIocToolboxDom();
    const { document } = dom.window;
    const styleText = document.querySelector('style')?.textContent ?? '';
    const groupsRule = styleText.match(/\.groups\s*\{[^}]+\}/)?.[0] ?? '';
    const groupRule = styleText.match(/\.group\s*\{[^}]+\}/)?.[0] ?? '';
    const groupBodyRule = styleText.match(/\.group-body\s*\{[^}]+\}/)?.[0] ?? '';

    expect(document.title).toContain('IOC Toolbox');
    expect(document.querySelector('html')?.lang).toBe('zh-CN');
    expect(document.querySelector('h1')?.textContent).toContain('IOC 工具箱');
    expect(document.querySelector('h2')?.textContent).toContain('混合输入');
    expect(document.getElementById('analyzeBtn')?.textContent).toContain('分析指标');
    expect(document.getElementById('copyJsonBtn')?.textContent).toContain('复制分组 JSON');
    expect(document.querySelector('.meta')?.textContent).toContain('支持直接粘贴多行');
    expect(document.getElementById('input')?.getAttribute('placeholder')).toContain('示例');
    expect(styleText).toContain('@media (max-width: 1100px)');
    expect(styleText).toContain('grid-template-columns: 1fr;');
    expect(groupsRule).toContain('display: flex;');
    expect(groupsRule).toContain('flex-direction: column;');
    expect(groupRule).toContain('flex: 0 0 auto;');
    expect(groupBodyRule).toContain('max-height');
    expect(groupBodyRule).toContain('overflow: auto;');

    dom.window.close();
  });

  it('extracts grouped items from large mixed-content input', () => {
    const dom = createIocToolboxDom();
    const { document } = dom.window;
    const input = document.getElementById('input');
    const analyzeButton = document.getElementById('analyzeBtn');

    input.value = [
      '告警备注：请优先处理以下 IOC',
      'https://portal.example.com/login?next=%2Fhome',
      '{"domain":"cdn.example.net","ip":"8.8.8.8","email":"SOC@example.org"}',
      'CVE-2025-12345；2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824',
      '补充：访问 hxxps://ignore.invalid 不需要识别，真正域名是 api.example.cn/path',
    ].join('\n');

    analyzeButton.click();

    const summaryLabels = Array.from(document.querySelectorAll('.summary-card .label')).map((node) =>
      node.textContent?.trim()
    );
    const groupLabels = Array.from(document.querySelectorAll('.group-header strong')).map((node) =>
      node.textContent?.trim()
    );
    const renderedItems = Array.from(document.querySelectorAll('.group-item-text')).map((node) =>
      node.textContent?.trim()
    );
    const statusText = document.getElementById('status')?.textContent ?? '';

    expect(statusText).toContain('已分析');
    expect(summaryLabels).toEqual(expect.arrayContaining(['原始片段', '唯一指标', '识别类别']));
    expect(groupLabels).toEqual(
      expect.arrayContaining(['URL', '域名', 'IPv4', '邮箱', 'CVE', 'SHA256'])
    );
    expect(renderedItems).toEqual(
      expect.arrayContaining([
        'https://portal.example.com/login?next=%2Fhome',
        'cdn.example.net',
        'api.example.cn',
        '8.8.8.8',
        'soc@example.org',
        'CVE-2025-12345',
        '2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824',
      ])
    );

    dom.window.close();
  });

  it('renders item-level copy actions so long IOC values remain directly operable', () => {
    const dom = createIocToolboxDom();
    const { document } = dom.window;
    const input = document.getElementById('input');
    const analyzeButton = document.getElementById('analyzeBtn');

    input.value = [
      '可疑摘要',
      '2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824',
    ].join('\n');

    analyzeButton.click();

    const copyButtons = Array.from(document.querySelectorAll('[data-copy-value]'));
    const copiedValues = copyButtons.map((node) => node.getAttribute('data-copy-value'));

    expect(copyButtons.length).toBeGreaterThan(0);
    expect(copiedValues).toContain('2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824');
    expect(document.body.textContent).toContain('复制此项');

    dom.window.close();
  });

  it('renders clearer summary metrics and collapses unknown content until needed', () => {
    const dom = createIocToolboxDom();
    const { document } = dom.window;
    const input = document.getElementById('input');
    const analyzeButton = document.getElementById('analyzeBtn');

    input.value = [
      'https://portal.example.com/login',
      'https://portal.example.com/login',
      'api.example.cn',
      'api.example.cn',
      '这是一段需要保留的工单备注',
    ].join('\n');

    analyzeButton.click();

    const summaryLabels = Array.from(document.querySelectorAll('.summary-card .label')).map((node) =>
      node.textContent?.trim()
    );
    const toggleUnknownButton = document.querySelector('[data-toggle-group="unknown"]');
    const unknownBody = document.querySelector('[data-group-body="unknown"]');

    expect(summaryLabels).toEqual(
      expect.arrayContaining(['规范化条目', '重复收敛', '未识别'])
    );
    expect(toggleUnknownButton?.textContent).toContain('展开');
    expect(unknownBody?.hasAttribute('hidden')).toBe(true);

    toggleUnknownButton?.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }));
    const expandedUnknownBody = document.querySelector('[data-group-body="unknown"]');
    expect(expandedUnknownBody?.hasAttribute('hidden')).toBe(false);

    dom.window.close();
  });

  it('exports structured grouped json, normalized lines, and group-level copy output', async () => {
    const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
    const dom = createIocToolboxDom({ clipboardWriteText });
    const { document } = dom.window;
    const input = document.getElementById('input');
    const analyzeButton = document.getElementById('analyzeBtn');
    const copyJsonButton = document.getElementById('copyJsonBtn');
    const copyFlatButton = document.getElementById('copyFlatBtn');

    input.value = [
      'https://portal.example.com/login',
      'api.example.cn',
      'SOC@example.org',
      '未识别备注',
    ].join('\n');

    analyzeButton.click();

    copyJsonButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    const groupedPayload = JSON.parse(clipboardWriteText.mock.calls[0][0]);
    expect(groupedPayload).toHaveProperty('summary');
    expect(groupedPayload).toHaveProperty('groupedIndicators');
    expect(groupedPayload).toHaveProperty('normalizedIndicators');
    expect(groupedPayload).toHaveProperty('unknownContent');

    copyFlatButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    expect(clipboardWriteText.mock.calls[1][0]).toContain('URL\thttps://portal.example.com/login');
    expect(clipboardWriteText.mock.calls[1][0]).toContain('域名\tapi.example.cn');

    const copyDomainGroupButton = document.querySelector('[data-copy-group="domain"]');
    copyDomainGroupButton?.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    expect(clipboardWriteText.mock.calls[2][0]).toBe('域名\tapi.example.cn');

    dom.window.close();
  });

  it('falls back to legacy copy when navigator clipboard is unavailable', async () => {
    const clipboardWriteText = vi.fn().mockRejectedValue(new Error('clipboard denied'));
    const dom = createIocToolboxDom({ clipboardWriteText });
    const { document } = dom.window;
    const execCommand = vi.fn().mockReturnValue(true);
    document.execCommand = execCommand;

    const input = document.getElementById('input');
    const analyzeButton = document.getElementById('analyzeBtn');
    const copyJsonButton = document.getElementById('copyJsonBtn');

    input.value = 'https://portal.example.com/login';
    analyzeButton.click();
    copyJsonButton.click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(clipboardWriteText).toHaveBeenCalled();
    expect(execCommand).toHaveBeenCalledWith('copy');
    expect(document.getElementById('status')?.textContent).toContain('已复制分组 JSON');

    dom.window.close();
  });
});
