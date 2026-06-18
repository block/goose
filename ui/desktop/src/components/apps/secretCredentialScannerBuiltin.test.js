/**
 * @vitest-environment jsdom
 */
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

const require = createRequire(import.meta.url);
const { JSDOM } = require('jsdom');
const SECRET_SCANNER_HTML_PATH = path.resolve(
  process.cwd(),
  '../../crates/goose/src/goose_apps/secret-credential-scanner.html'
);

function createSecretScannerDom({ clipboardWriteText } = {}) {
  const html = fs.readFileSync(SECRET_SCANNER_HTML_PATH, 'utf8');
  const dom = new JSDOM(html, {
    runScripts: 'dangerously',
    url: 'https://security-goose.local/ui://apps/secret-credential-scanner',
  });

  Object.defineProperty(dom.window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: clipboardWriteText ?? vi.fn().mockResolvedValue(undefined),
    },
  });

  return dom;
}

describe('Secret Scanner built-in app', () => {
  it('renders a Chinese-first offline secret scanning interface', () => {
    const dom = createSecretScannerDom();
    const { document } = dom.window;
    const html = fs.readFileSync(SECRET_SCANNER_HTML_PATH, 'utf8');

    expect(document.querySelector('html')?.lang).toBe('zh-CN');
    expect(document.title).toContain('敏感');
    expect(document.querySelector('h1')?.textContent).toContain('敏感');
    expect(document.body.textContent).toContain('混合输入');
    expect(document.getElementById('analyzeBtn')?.textContent).toContain('扫描敏感信息');
    expect(document.getElementById('copyJsonBtn')?.textContent).toContain('复制结构化 JSON');
    expect(document.getElementById('copyFlatBtn')?.textContent).toContain('复制规范化列表');
    expect(html).not.toContain('summary-card');
    expect(html).not.toContain('recommendations');
    expect(html).not.toContain('class="panel"');

    dom.window.close();
  });

  it('extracts common cloud and collaboration secrets from mixed content', () => {
    const dom = createSecretScannerDom();
    const { document } = dom.window;

    document.getElementById('input').value = [
      'Authorization: Bearer sk-live-abc1234567890',
      'JWT=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.signature',
      '腾讯云 SecretId=AKID1234567890abcdefghijklmn',
      '阿里云 AccessKeyId=LTAI5t8AbCdEfGh12345',
      'HUAWEI_CLOUD_AK=ABCD1234EFGH5678IJKL',
      'corpsecret=wEcOmSecret123456',
      'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=12345678-1234-1234-1234-1234567890ab',
      'https://oapi.dingtalk.com/robot/send?access_token=abcdef1234567890abcdef1234567890',
      'mysql://analyst:SuperSecret!@db.example.com:3306/app',
      '-----BEGIN PRIVATE KEY-----\\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\\n-----END PRIVATE KEY-----',
    ].join('\\n');

    document.getElementById('analyzeBtn').click();

    const bodyText = document.body.textContent ?? '';
    const summaryLabels = Array.from(document.querySelectorAll('.summary-item .label')).map((node) =>
      node.textContent?.trim()
    );

    expect(summaryLabels).toEqual(
      expect.arrayContaining(['原始命中', '唯一敏感项', '识别类别', '高风险'])
    );
    expect(bodyText).toContain('Bearer Token');
    expect(bodyText).toContain('JWT');
    expect(bodyText).toContain('腾讯云凭据');
    expect(bodyText).toContain('阿里云凭据');
    expect(bodyText).toContain('华为云凭据');
    expect(bodyText).toContain('企微');
    expect(bodyText).toContain('钉钉');
    expect(bodyText).toContain('数据库连接串');
    expect(bodyText).toContain('私钥 / PEM');
    expect(document.getElementById('recommendations')?.textContent ?? '').toBe('');

    expect(document.querySelectorAll('.result-section').length).toBeGreaterThan(0);
    expect(document.querySelectorAll('.result-row').length).toBeGreaterThan(0);

    dom.window.close();
  });

  it('supports structured export and clipboard fallback', async () => {
    const clipboardWriteText = vi.fn().mockRejectedValue(new Error('clipboard denied'));
    const dom = createSecretScannerDom({ clipboardWriteText });
    const { document } = dom.window;
    document.execCommand = vi.fn().mockReturnValue(true);

    document.getElementById('input').value = 'Authorization: Bearer sk-live-abc1234567890';
    document.getElementById('analyzeBtn').click();

    document.getElementById('copyJsonBtn').click();
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
    await new Promise((resolve) => dom.window.setTimeout(resolve, 0));

    expect(clipboardWriteText).toHaveBeenCalled();
    const exportedJson = JSON.parse(clipboardWriteText.mock.calls[0][0]);
    expect(exportedJson).not.toHaveProperty('recommendations');
    expect(exportedJson.groupedFindings[0]).not.toHaveProperty('recommendation');
    expect(document.execCommand).toHaveBeenCalledWith('copy');
    expect(document.getElementById('status')?.textContent).toContain('已复制');

    dom.window.close();
  });
});
