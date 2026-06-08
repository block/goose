import { describe, it, expect } from 'vitest';
import { OPEN_FILE_PROTOCOL } from './linkifyPaths';

const UNIX_PATH_RE = /(?:^|[\s('"`[(,;]|\/\*.*?\*\/)?(\/(?:[a-zA-Z0-9._+-]+\/){1,}[a-zA-Z0-9._+-]+)/g;
const TILDE_PATH_RE = /(?:^|[\s('"`[(,;]|\/\*.*?\*\/)?(~\/(?:[a-zA-Z0-9._+-]+\/)*[a-zA-Z0-9._+-]+)/g;
const WIN_PATH_RE = /(?:^|[\s('"`[(,;]|\/\*.*?\*\/)?([A-Za-z]:[\\/](?:[a-zA-Z0-9._+-]+[\\/])+[a-zA-Z0-9._+-]+)/g;

type PathMatch = [index: number, path: string];

function findPaths(text: string): PathMatch[] {
  const matches: PathMatch[] = [];
  for (const re of [UNIX_PATH_RE, TILDE_PATH_RE, WIN_PATH_RE]) {
    let m: RegExpExecArray | null;
    const localRe = new RegExp(re.source, 'g');
    while ((m = localRe.exec(text)) !== null) {
      const prefixLen = m[1].length;
      const path = m[2];
      const index = m.index + prefixLen;
      matches.push([index, path]);
    }
  }
  matches.sort((a, b) => a[0] - b[0]);
  const result: PathMatch[] = [];
  let lastEnd = 0;
  for (const [index, path] of matches) {
    if (index < lastEnd) continue;
    result.push([index, path]);
    lastEnd = index + path.length;
  }
  return result;
}

describe('path linkification', () => {
  describe('Unix absolute paths', () => {
    it('detects Unix absolute paths', () => {
      const matches = findPaths('Created file at /home/user/project/src/main.rs');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/home/user/project/src/main.rs');
    });

    it('does not match single-segment paths', () => {
      const matches = findPaths('Go to / for root');
      expect(matches).toHaveLength(0);
    });

    it('detects multiple paths in one line', () => {
      const matches = findPaths('Compare /etc/hosts and /etc/resolv.conf');
      expect(matches).toHaveLength(2);
      expect(matches[0][1]).toBe('/etc/hosts');
      expect(matches[1][1]).toBe('/etc/resolv.conf');
    });
  });

  describe('Tilde paths', () => {
    it('detects tilde paths', () => {
      const matches = findPaths('Config at ~/.config/app/settings.json');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('~/.config/app/settings.json');
    });

    it('does not match bare tilde', () => {
      const matches = findPaths('Go to ~ for home');
      expect(matches).toHaveLength(0);
    });
  });

  describe('Windows paths', () => {
    it('detects Windows paths', () => {
      const matches = findPaths('File at C:\\Users\\dev\\project\\index.ts');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('C:\\Users\\dev\\project\\index.ts');
    });
  });

  describe('Edge cases', () => {
    it('detects paths after punctuation', () => {
      const matches = findPaths('Saved to "/var/log/app.log"');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/var/log/app.log');
    });

    it('detects paths with dots and underscores', () => {
      const matches = findPaths('Read /home/user/.env.local');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/home/user/.env.local');
    });

    it('does not match URLs', () => {
      const matches = findPaths('Visit https://example.com/page for info');
      expect(matches).toHaveLength(0);
    });

    it('generates correct open-file URLs', () => {
      const path = '/home/user/project/src/main.rs';
      expect(OPEN_FILE_PROTOCOL + path).toBe('open-file:///home/user/project/src/main.rs');
    });

    it('handles paths with hyphens', () => {
      const matches = findPaths('Check /usr/local/lib/my-app/config.yaml');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/usr/local/lib/my-app/config.yaml');
    });

    it('does not match content inside code blocks', () => {
      const matches = findPaths('Use `Array<T>` for generics');
      expect(matches).toHaveLength(0);
    });

    it('matches paths inside inline code markers', () => {
      const matches = findPaths('`/usr/local/bin/node`');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/usr/local/bin/node');
    });
  });
});