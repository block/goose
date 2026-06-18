import { describe, it, expect } from 'vitest';
import type { Root } from 'mdast';
import { findPaths, OPEN_FILE_PROTOCOL, remarkLinkifyPaths } from './linkifyPaths';

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

    it('detects paths with spaces in segment names', () => {
      const matches = findPaths('Saved /Users/me/My Project/result.txt for review');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/My Project/result.txt');
    });

    it('detects lowercase folder names with spaces at end of path', () => {
      const matches = findPaths('Output in /home/user/my documents');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/home/user/my documents');
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

    it('detects Windows paths with spaces in segment names', () => {
      const matches = findPaths('File at C:\\Users\\dev\\My Project\\index.ts');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('C:\\Users\\dev\\My Project\\index.ts');
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

    it('strips trailing sentence punctuation', () => {
      const matches = findPaths('Created /tmp/result.txt.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/result.txt');
    });

    it('does not include trailing prose before sentence punctuation', () => {
      const matches = findPaths('Created /tmp/result successfully.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/result');
    });

    it('does not extend short single-char basenames with prose', () => {
      const matches = findPaths('Created /tmp/a successfully.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/a');
    });

    it('includes spaced folder names before sentence punctuation', () => {
      const matches = findPaths('Saved to /home/user/my documents.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/home/user/my documents');
    });

    it('does not match URLs', () => {
      const matches = findPaths('Visit https://example.com/page for info');
      expect(matches).toHaveLength(0);
    });

    it('generates correct open-file URLs', () => {
      const path = '/home/user/project/src/main.rs';
      expect(OPEN_FILE_PROTOCOL + encodeURI(path)).toBe(
        'open-file:///home/user/project/src/main.rs'
      );
    });

    it('encodes spaces in open-file URLs', () => {
      const path = '/Users/me/My Project/result.txt';
      expect(OPEN_FILE_PROTOCOL + encodeURI(path)).toBe(
        'open-file:///Users/me/My%20Project/result.txt'
      );
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

    it('detects paths with parenthesized download suffixes', () => {
      const matches = findPaths('Saved /Users/me/Downloads/report (1).pdf');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/Downloads/report (1).pdf');
    });

    it('preserves closing parens in parenthesized paths without extension', () => {
      const matches = findPaths('Saved /Users/me/Downloads/report (1)');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/Downloads/report (1)');
    });

    it('strips sentence punctuation after parenthesized paths', () => {
      const matches = findPaths('Saved /Users/me/Downloads/report (1).');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/Downloads/report (1)');
    });

    it('detects paths with commas in filenames', () => {
      const matches = findPaths('Saved /tmp/report,final.txt');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/report,final.txt');
    });

    it('does not extend paths across comma-separated prose', () => {
      const matches = findPaths('Created /tmp/report, then continued');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/report');
    });

    it('detects paths with numeric suffixes in segment names', () => {
      const matches = findPaths('Saved /Users/me/Project 2026.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/Project 2026');
    });

    it('detects paths with Unicode segment names', () => {
      const matches = findPaths('Saved /Users/me/デスクトップ/out.txt');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/デスクトップ/out.txt');
    });

    it('does not linkify partial paths before unsupported characters', () => {
      const matches = findPaths('See /Users/me/🎉/out.txt');
      expect(matches).toHaveLength(0);
    });

    it('returns no matches for long prose without paths', () => {
      const prose = 'word '.repeat(10_000);
      const start = Date.now();
      const matches = findPaths(prose);
      const elapsed = Date.now() - start;

      expect(matches).toHaveLength(0);
      expect(elapsed).toBeLessThan(500);
    });
  });
});

describe('remarkLinkifyPaths', () => {
  it('does not linkify paths inside link references', () => {
    const tree: Root = {
      type: 'root',
      children: [
        {
          type: 'paragraph',
          children: [
            {
              type: 'linkReference',
              identifier: 'log',
              label: 'log',
              referenceType: 'full',
              children: [{ type: 'text', value: 'log /tmp/out' }],
            },
          ],
        },
      ],
    };

    const transform = (remarkLinkifyPaths as unknown as () => (tree: Root) => void)();
    transform(tree);

    const paragraph = tree.children[0];
    expect(paragraph.type).toBe('paragraph');
    if (paragraph.type !== 'paragraph') return;

    const linkRef = paragraph.children[0];
    expect(linkRef.type).toBe('linkReference');
    if (linkRef.type !== 'linkReference') return;

    expect(linkRef.children).toHaveLength(1);
    expect(linkRef.children[0]).toEqual({ type: 'text', value: 'log /tmp/out' });
  });

  it('does not linkify paths inside formatted link labels', () => {
    const tree: Root = {
      type: 'root',
      children: [
        {
          type: 'paragraph',
          children: [
            {
              type: 'link',
              url: 'https://example.com',
              children: [
                {
                  type: 'strong',
                  children: [{ type: 'text', value: '/tmp/out' }],
                },
              ],
            },
          ],
        },
      ],
    };

    const transform = (remarkLinkifyPaths as unknown as () => (tree: Root) => void)();
    transform(tree);

    const paragraph = tree.children[0];
    expect(paragraph.type).toBe('paragraph');
    if (paragraph.type !== 'paragraph') return;

    const link = paragraph.children[0];
    expect(link.type).toBe('link');
    if (link.type !== 'link') return;

    expect(link.url).toBe('https://example.com');
    expect(link.children).toHaveLength(1);
    expect(link.children[0].type).toBe('strong');
    if (link.children[0].type !== 'strong') return;
    expect(link.children[0].children[0]).toEqual({ type: 'text', value: '/tmp/out' });
  });
});
