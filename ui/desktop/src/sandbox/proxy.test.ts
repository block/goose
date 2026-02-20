import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  normalizeDomain,
  isIPAddress,
  isLoopback,
  matchesBlocked,
  checkBlocked,
  loadBlocked,
  parseConnectTarget,
  type ProxyOptions,
} from './proxy';

describe('parseConnectTarget', () => {
  it('parses host:port', () => {
    expect(parseConnectTarget('example.com:443')).toEqual({ host: 'example.com', port: 443 });
  });

  it('parses host:port with non-standard port', () => {
    expect(parseConnectTarget('api.internal:8443')).toEqual({ host: 'api.internal', port: 8443 });
  });

  it('parses bracketed IPv6 with port', () => {
    expect(parseConnectTarget('[2001:db8::1]:443')).toEqual({ host: '2001:db8::1', port: 443 });
    expect(parseConnectTarget('[::1]:8080')).toEqual({ host: '::1', port: 8080 });
  });

  it('rejects invalid targets', () => {
    expect(parseConnectTarget(':443')).toEqual({ host: '', port: 0 });
    expect(parseConnectTarget('')).toEqual({ host: '', port: 0 });
    expect(parseConnectTarget('example.com')).toEqual({ host: '', port: 0 });
    expect(parseConnectTarget('example.com:0')).toEqual({ host: '', port: 0 });
    expect(parseConnectTarget('example.com:99999')).toEqual({ host: '', port: 0 });
    expect(parseConnectTarget('example.com:abc')).toEqual({ host: '', port: 0 });
  });
});

describe('normalizeDomain', () => {
  it('lowercases and trims', () => {
    expect(normalizeDomain('GitHub.COM')).toBe('github.com');
    expect(normalizeDomain('  example.com  ')).toBe('example.com');
  });

  it('strips trailing dot', () => {
    expect(normalizeDomain('example.com.')).toBe('example.com');
  });

  it('strips IPv6 brackets', () => {
    expect(normalizeDomain('[::1]')).toBe('::1');
  });

  it('handles punycode via URL constructor', () => {
    expect(normalizeDomain('MÜNCHEN.de')).toBe(new URL('http://münchen.de').hostname);
  });

  it('handles plain domain', () => {
    expect(normalizeDomain('api.example.com')).toBe('api.example.com');
  });
});

describe('isIPAddress', () => {
  it('detects IPv4', () => {
    expect(isIPAddress('192.168.1.1')).toBe(true);
    expect(isIPAddress('10.0.0.1')).toBe(true);
    expect(isIPAddress('127.0.0.1')).toBe(true);
  });

  it('detects IPv6', () => {
    expect(isIPAddress('::1')).toBe(true);
    expect(isIPAddress('2001:db8::1')).toBe(true);
  });

  it('rejects domains', () => {
    expect(isIPAddress('example.com')).toBe(false);
    expect(isIPAddress('localhost')).toBe(false);
  });
});

describe('isLoopback', () => {
  it('matches loopback addresses', () => {
    expect(isLoopback('localhost')).toBe(true);
    expect(isLoopback('LOCALHOST')).toBe(true);
    expect(isLoopback('127.0.0.1')).toBe(true);
    expect(isLoopback('127.255.255.255')).toBe(true);
    expect(isLoopback('::1')).toBe(true);
    expect(isLoopback('[::1]')).toBe(true);
  });

  it('rejects non-loopback', () => {
    expect(isLoopback('192.168.1.1')).toBe(false);
    expect(isLoopback('example.com')).toBe(false);
  });
});

describe('matchesBlocked', () => {
  const blocked = new Set(['evil.com', 'pastebin.com', 'bad.example.org']);

  it('blocks exact domain and subdomains', () => {
    expect(matchesBlocked('evil.com', blocked)).toBe(true);
    expect(matchesBlocked('www.evil.com', blocked)).toBe(true);
    expect(matchesBlocked('deep.sub.evil.com', blocked)).toBe(true);
  });

  it('allows non-blocked domains', () => {
    expect(matchesBlocked('github.com', blocked)).toBe(false);
    expect(matchesBlocked('example.com', blocked)).toBe(false);
  });

  it('does not block parent of blocked domain', () => {
    expect(matchesBlocked('example.org', blocked)).toBe(false);
    expect(matchesBlocked('com', blocked)).toBe(false);
  });

  it('is case-insensitive and handles trailing dot', () => {
    expect(matchesBlocked('EVIL.COM', blocked)).toBe(true);
    expect(matchesBlocked('evil.com.', blocked)).toBe(true);
  });

  it('handles empty blocklist', () => {
    expect(matchesBlocked('anything.com', new Set())).toBe(false);
  });
});

describe('loadBlocked', () => {
  it('returns empty set for undefined or missing path', () => {
    expect(loadBlocked(undefined).size).toBe(0);
    expect(loadBlocked('/nonexistent/path/blocked.txt').size).toBe(0);
  });

  it('loads domains from file, skipping comments and blanks', () => {
    const tmpFile = path.join(os.tmpdir(), `blocked-test-${Date.now()}.txt`);
    fs.writeFileSync(
      tmpFile,
      `# comment
evil.com
  pastebin.com  

# another comment
transfer.sh
`
    );
    try {
      const result = loadBlocked(tmpFile);
      expect(result.size).toBe(3);
      expect(result.has('evil.com')).toBe(true);
      expect(result.has('pastebin.com')).toBe(true);
      expect(result.has('transfer.sh')).toBe(true);
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });
});

describe('checkBlocked', () => {
  const blocked = new Set(['evil.com', 'pastebin.com']);
  const noLD = undefined;
  const noLDCache = undefined;
  const defaultOptions: ProxyOptions = {};

  it('allows normal HTTPS traffic', async () => {
    const result = await checkBlocked('github.com', 443, blocked, noLD, noLDCache, defaultOptions);
    expect(result.blocked).toBe(false);
  });

  it('blocks domains and subdomains on the blocklist', async () => {
    const exact = await checkBlocked('evil.com', 443, blocked, noLD, noLDCache, defaultOptions);
    expect(exact).toEqual({ blocked: true, reason: 'blocklist' });

    const sub = await checkBlocked('api.evil.com', 443, blocked, noLD, noLDCache, defaultOptions);
    expect(sub).toEqual({ blocked: true, reason: 'blocklist' });
  });

  it('blocks raw IP addresses by default, allows when opted in', async () => {
    const blocked_ = await checkBlocked(
      '93.184.216.34',
      443,
      blocked,
      noLD,
      noLDCache,
      defaultOptions
    );
    expect(blocked_).toEqual({ blocked: true, reason: 'ip-address' });

    const allowed = await checkBlocked('93.184.216.34', 443, blocked, noLD, noLDCache, {
      allowIPAddresses: true,
    });
    expect(allowed.blocked).toBe(false);
  });

  it('does not block loopback by default, blocks when opted in', async () => {
    const allowed = await checkBlocked('localhost', 8080, blocked, noLD, noLDCache, defaultOptions);
    expect(allowed.blocked).toBe(false);

    const blocked_ = await checkBlocked('localhost', 8080, blocked, noLD, noLDCache, {
      blockLoopback: true,
    });
    expect(blocked_).toEqual({ blocked: true, reason: 'loopback' });

    const blocked127 = await checkBlocked('127.0.0.1', 8080, blocked, noLD, noLDCache, {
      blockLoopback: true,
    });
    expect(blocked127).toEqual({ blocked: true, reason: 'loopback' });
  });

  it('allows SSH to default git hosts', async () => {
    for (const host of ['github.com', 'gitlab.com', 'bitbucket.org', 'ssh.dev.azure.com']) {
      const result = await checkBlocked(host, 22, blocked, noLD, noLDCache, defaultOptions);
      expect(result.blocked).toBe(false);
    }
  });

  it('blocks SSH to non-git hosts on all SSH ports', async () => {
    for (const port of [22, 2222, 7999]) {
      const result = await checkBlocked(
        'random-server.com',
        port,
        blocked,
        noLD,
        noLDCache,
        defaultOptions
      );
      expect(result).toEqual({ blocked: true, reason: 'ssh-non-git-host' });
    }
  });

  it('blocks all SSH when allowSSH is false', async () => {
    const result = await checkBlocked('github.com', 22, blocked, noLD, noLDCache, {
      allowSSH: false,
    });
    expect(result).toEqual({ blocked: true, reason: 'ssh-disabled' });
  });

  it('allows SSH to any host when allowSSHToAllHosts is true', async () => {
    const result = await checkBlocked('random-server.com', 22, blocked, noLD, noLDCache, {
      allowSSHToAllHosts: true,
    });
    expect(result.blocked).toBe(false);
  });

  it('respects custom git hosts list', async () => {
    const opts = { gitHosts: ['gitea.internal.com'] };
    const allowed = await checkBlocked('gitea.internal.com', 22, blocked, noLD, noLDCache, opts);
    expect(allowed.blocked).toBe(false);

    const denied = await checkBlocked('github.com', 22, blocked, noLD, noLDCache, opts);
    expect(denied).toEqual({ blocked: true, reason: 'ssh-non-git-host' });
  });

  it('SSH rules only apply to SSH ports', async () => {
    const result = await checkBlocked(
      'random-server.com',
      443,
      blocked,
      noLD,
      noLDCache,
      defaultOptions
    );
    expect(result.blocked).toBe(false);
  });

  it('checks blocking layers in priority order', async () => {
    // loopback before IP
    const loopback = await checkBlocked('127.0.0.1', 443, blocked, noLD, noLDCache, {
      blockLoopback: true,
      allowIPAddresses: false,
    });
    expect(loopback.reason).toBe('loopback');

    // blocklist before SSH
    const blocklist = await checkBlocked('evil.com', 22, blocked, noLD, noLDCache, defaultOptions);
    expect(blocklist.reason).toBe('blocklist');
  });
});
