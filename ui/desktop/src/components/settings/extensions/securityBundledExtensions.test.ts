import { describe, expect, it } from 'vitest';

import bundledExtensions from './bundled-extensions.json';

const SECURITY_EXTENSION_IDS = [
  'aiseesec-mcp',
  'local-security-gateway-mcp',
  'threat-intel-mcp',
  'browser-assist-mcp',
];

describe('security bundled extension catalog', () => {
  it('includes the first four security preview extensions as disabled stdio entries', () => {
    const entries = bundledExtensions.filter((entry) => SECURITY_EXTENSION_IDS.includes(entry.id));

    expect(entries.map((entry) => entry.id)).toEqual(SECURITY_EXTENSION_IDS);
    expect(entries.every((entry) => entry.type === 'stdio')).toBe(true);
    expect(entries.every((entry) => entry.enabled === false)).toBe(true);
    expect(entries.every((entry) => entry.cmd === 'node')).toBe(true);
  });

  it('keeps node as the source catalog command and relies on desktop sync to resolve the actual runner', () => {
    const threatIntel = bundledExtensions.find((entry) => entry.id === 'threat-intel-mcp');
    const browserAssist = bundledExtensions.find((entry) => entry.id === 'browser-assist-mcp');

    expect(threatIntel?.args).toEqual(['distro/security-cn/extensions/threat-intel-mcp/server.mjs']);
    expect(browserAssist?.args).toEqual([
      'distro/security-cn/extensions/browser-assist-mcp/server.mjs',
    ]);
    expect(threatIntel?.cmd).toBe('node');
    expect(browserAssist?.cmd).toBe('node');
  });

  it('marks browser-assist and threat-intel as zero-config local preview entries', () => {
    const browserAssist = bundledExtensions.find((entry) => entry.id === 'browser-assist-mcp');
    const threatIntel = bundledExtensions.find((entry) => entry.id === 'threat-intel-mcp');

    expect(browserAssist?.description).toContain('本机网页快照');
    expect(browserAssist?.env_keys).toEqual([]);
    expect(threatIntel?.description).toContain('本机 IOC 提取');
    expect(threatIntel?.env_keys).toEqual([]);
  });
});
