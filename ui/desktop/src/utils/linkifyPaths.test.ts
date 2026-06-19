import { describe, it, expect } from 'vitest';
import type { Root } from 'mdast';
import { findPaths, OPEN_FILE_PROTOCOL, remarkLinkifyPaths, isTrustedGeneratedFileLink } from './linkifyPaths';

describe('path linkification', () => {
  describe('Unix absolute paths', () => {
    it('detects Unix absolute paths', () => {
      const matches = findPaths('Created file at /home/user/project/src/main.rs');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/home/user/project/src/main.rs');
    });

    it('does not match bare root path', () => {
      const matches = findPaths('Go to / for root');
      expect(matches).toHaveLength(0);
    });

    it('detects top-level absolute directories', () => {
      expect(findPaths('Saved to /tmp')[0][1]).toBe('/tmp');
      expect(findPaths('Saved to /workspace')[0][1]).toBe('/workspace');
    });

    it('detects paths in questions with trailing question marks', () => {
      expect(findPaths('Can you check /tmp/out?')[0][1]).toBe('/tmp/out');
    });

    it('detects single-segment Windows absolute directories', () => {
      expect(findPaths('See C:\\Users for details')[0][1]).toBe('C:\\Users');
      expect(findPaths('See C:\\Windows for details')[0][1]).toBe('C:\\Windows');
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

    it('does not treat prose labels as Windows paths', () => {
      expect(findPaths('Option A:retry')).toHaveLength(0);
      expect(findPaths('Status C:passed')).toHaveLength(0);
    });
  });

  describe('Edge cases', () => {
    it('detects paths after punctuation', () => {
      const matches = findPaths('Saved to "/var/log/app.log"');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/var/log/app.log');
    });

    it('detects paths wrapped in parentheses or brackets', () => {
      expect(findPaths('See (/tmp/out)')[0][1]).toBe('/tmp/out');
      expect(findPaths('See (C:\\Users\\dev\\file.txt)')[0][1]).toBe('C:\\Users\\dev\\file.txt');
      expect(findPaths('See [/tmp/out]')[0][1]).toBe('/tmp/out');
    });

    it('detects paths with brackets and question marks in filenames', () => {
      expect(findPaths('Saved /tmp/report[1].pdf')[0][1]).toBe('/tmp/report[1].pdf');
      expect(findPaths('Saved /tmp/what?now.txt')[0][1]).toBe('/tmp/what?now.txt');
    });

    it('detects paths with bracketed suffixes without extension', () => {
      expect(findPaths('Saved /tmp/report[1]')[0][1]).toBe('/tmp/report[1]');
    });

    it('detects paths with inline parenthesized suffixes in filenames', () => {
      expect(findPaths('Saved /tmp/report(1).pdf')[0][1]).toBe('/tmp/report(1).pdf');
      expect(findPaths('Saved /tmp/foo(bar).txt')[0][1]).toBe('/tmp/foo(bar).txt');
    });

    it('does not absorb inline parenthetical prose without extension', () => {
      expect(findPaths('Created /tmp/out(temporary) for debugging')[0][1]).toBe('/tmp/out');
      expect(findPaths('Created /tmp/report(temp) for debugging')[0][1]).toBe('/tmp/report');
    });

    it('detects paths with bracketed suffixes after spaced filename segments', () => {
      expect(findPaths('Saved /tmp/My report[1].pdf')[0][1]).toBe('/tmp/My report[1].pdf');
    });

    it('detects paths with long lowercase spaced filename segments', () => {
      expect(findPaths('Saved /tmp/project notes.txt')[0][1]).toBe('/tmp/project notes.txt');
    });

    it('detects extension-bearing spaced filenames after titlecase basenames', () => {
      expect(findPaths('Saved /tmp/Project notes.txt')[0][1]).toBe('/tmp/Project notes.txt');
    });

    it('detects extension-bearing spaced filenames after acronym basenames', () => {
      expect(findPaths('Saved /tmp/API response.json')[0][1]).toBe('/tmp/API response.json');
    });

    it('detects multi-dot extension-bearing spaced filenames', () => {
      expect(findPaths('Saved /tmp/My draft.final.md')[0][1]).toBe('/tmp/My draft.final.md');
    });

    it('detects paths with multiple lowercase spaced filename words', () => {
      expect(findPaths('Saved /tmp/project notes draft.txt')[0][1]).toBe(
        '/tmp/project notes draft.txt'
      );
      expect(findPaths('Saved /tmp/project notes draft.txt.')[0][1]).toBe(
        '/tmp/project notes draft.txt'
      );
    });

    it('detects paths with uppercase extension words after spaced segments', () => {
      expect(findPaths('Saved /tmp/project notes Draft.txt')[0][1]).toBe(
        '/tmp/project notes Draft.txt'
      );
      expect(findPaths('Saved /tmp/project notes FINAL.pdf')[0][1]).toBe(
        '/tmp/project notes FINAL.pdf'
      );
      expect(findPaths('Saved /tmp/My Project Report.DOCX')[0][1]).toBe(
        '/tmp/My Project Report.DOCX'
      );
    });

    it('detects multi-word spaced filenames after titlecase basenames', () => {
      expect(findPaths('Saved /tmp/Project notes draft.txt')[0][1]).toBe(
        '/tmp/Project notes draft.txt'
      );
      expect(findPaths('Saved /tmp/Project notes draft.txt.')[0][1]).toBe(
        '/tmp/Project notes draft.txt'
      );
      expect(findPaths('Saved /tmp/Project notes Draft.txt')[0][1]).toBe(
        '/tmp/Project notes Draft.txt'
      );
    });

    it('detects multi-word spaced filenames after acronym basenames', () => {
      expect(findPaths('Saved /tmp/API response data.json')[0][1]).toBe(
        '/tmp/API response data.json'
      );
      expect(findPaths('Saved /tmp/API response data.json.')[0][1]).toBe(
        '/tmp/API response data.json'
      );
      expect(findPaths('Saved /tmp/API response DATA.json')[0][1]).toBe(
        '/tmp/API response DATA.json'
      );
    });

    it('detects paths with mixed-case multi-word filenames ending in extension', () => {
      expect(findPaths('Saved /tmp/project notes draft Final.txt')[0][1]).toBe(
        '/tmp/project notes draft Final.txt'
      );
      expect(findPaths('Saved /tmp/project notes Draft.txt.')[0][1]).toBe(
        '/tmp/project notes Draft.txt'
      );
    });

    it('strips trailing line and column suffixes from paths', () => {
      expect(findPaths('error at /workspace/src/lib.rs:42:7')[0][1]).toBe('/workspace/src/lib.rs');
      expect(findPaths('error at /workspace/src/lib.rs:42')[0][1]).toBe('/workspace/src/lib.rs');
      expect(findPaths('error at /workspace/goose/Cargo.toml:12:5')[0][1]).toBe(
        '/workspace/goose/Cargo.toml'
      );
      expect(findPaths('See /project/README.md:3 for details')[0][1]).toBe('/project/README.md');
    });

    it('strips line and column suffixes from extensionless diagnostic filenames', () => {
      expect(findPaths('error at /workspace/Dockerfile:12')[0][1]).toBe('/workspace/Dockerfile');
      expect(findPaths('error at /workspace/Makefile:8:1')[0][1]).toBe('/workspace/Makefile');
      expect(findPaths('See /project/README:5 for details')[0][1]).toBe('/project/README');
      expect(findPaths('error at /crates/goose-sdk/justfile:12')[0][1]).toBe(
        '/crates/goose-sdk/justfile'
      );
      expect(findPaths('error at /repo/dockerfile:3:1')[0][1]).toBe('/repo/dockerfile');
      expect(findPaths('error at /repo/makefile:42')[0][1]).toBe('/repo/makefile');
      expect(findPaths('error at /ci/Jenkinsfile:7')[0][1]).toBe('/ci/Jenkinsfile');
      expect(findPaths('error at /app/Procfile:2')[0][1]).toBe('/app/Procfile');
    });

    it('preserves colon-number suffixes in filenames', () => {
      expect(findPaths('Saved /tmp/snapshot:1')[0][1]).toBe('/tmp/snapshot:1');
      expect(findPaths('Saved /tmp/2026-06-18T18:30:00')[0][1]).toBe('/tmp/2026-06-18T18:30:00');
    });

    it('detects paths followed by tab-separated text', () => {
      expect(findPaths('See /tmp/out\tOK')[0][1]).toBe('/tmp/out');
    });

    it('detects paths with spaced bracket suffixes', () => {
      expect(findPaths('Saved /tmp/log [draft]')[0][1]).toBe('/tmp/log [draft]');
    });

    it('does not absorb bracket status annotations after paths', () => {
      expect(findPaths('Created /tmp/out [OK] for upload')[0][1]).toBe('/tmp/out');
    });

    it('does not absorb explanatory parentheticals after paths', () => {
      expect(findPaths('Created /tmp/out (temporary) for debugging')[0][1]).toBe('/tmp/out');
      expect(findPaths('Created /tmp/out (temp) for debugging')[0][1]).toBe('/tmp/out');
    });

    it('does not absorb parentheticals followed only by sentence punctuation', () => {
      expect(findPaths('Created /tmp/out (temporary).')[0][1]).toBe('/tmp/out');
      expect(findPaths('Created /tmp/report (temp).')[0][1]).toBe('/tmp/report');
      expect(findPaths('Created /tmp/out(temporary).')[0][1]).toBe('/tmp/out');
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

    it('does not extend short basenames with following prose', () => {
      const matches = findPaths('See /tmp/my for details');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/my');
    });

    it('does not extend short basenames with generic nouns', () => {
      expect(findPaths('Check /tmp/log file for details')[0][1]).toBe('/tmp/log');
    });

    it('does not extend paths through connective prose before another slash', () => {
      const matches = findPaths('Review /tmp/output and/or /tmp/logs');
      expect(matches[0][1]).toBe('/tmp/output');
      expect(matches[1][1]).toBe('/tmp/logs');
    });

    it('detects paths with spaced directory names before separators', () => {
      expect(findPaths('/Users/me/Library/Application Support/Goose')[0][1]).toBe(
        '/Users/me/Library/Application Support/Goose'
      );
      expect(findPaths('See C:\\Program Files\\Goose')[0][1]).toBe('C:\\Program Files\\Goose');
    });

    it('detects paths with parenthesized spaced directory names before separators', () => {
      expect(findPaths('See C:\\Program Files (x86)\\Goose')[0][1]).toBe(
        'C:\\Program Files (x86)\\Goose'
      );
      expect(findPaths('/Users/me/Apps (Beta)/app')[0][1]).toBe('/Users/me/Apps (Beta)/app');
    });

    it('includes spaced folder names before sentence punctuation', () => {
      const matches = findPaths('Saved to /home/user/my documents.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/home/user/my documents');
    });

    it('includes spaced folder names before newline terminators', () => {
      expect(findPaths('Saved /home/user/my documents\nDone')[0][1]).toBe(
        '/home/user/my documents'
      );
    });

    it('detects paths after assignment separators', () => {
      expect(findPaths('artifact=/tmp/out.log')[0][1]).toBe('/tmp/out.log');
      expect(findPaths('--output=/tmp/out')[0][1]).toBe('/tmp/out');
    });

    it('does not linkify paths inside URL query values', () => {
      expect(findPaths('See `https://host/download?file=/tmp/out`')).toHaveLength(0);
      expect(findPaths('Visit example.com?file=/tmp/out')).toHaveLength(0);
      expect(findPaths('Visit example.com?artifact=/tmp/out.log')).toHaveLength(0);
      expect(findPaths('Visit example.com?path=/tmp/out&other=1')).toHaveLength(0);
    });

    it('does not linkify paths inside URL query values with bracketed keys', () => {
      expect(findPaths('See https://host/download?file[]=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/download?items[0]=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/download?data[key]=/tmp/out.log')).toHaveLength(0);
      expect(findPaths('Visit example.com?file[]=/tmp/out&other=1')).toHaveLength(0);
    });

    it('does not linkify paths inside URL query values with percent-encoded keys', () => {
      expect(findPaths('See https://host/download?file%5B%5D=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/download?items%5B0%5D=/tmp/out.log')).toHaveLength(0);
      expect(findPaths('Visit example.com?file%5B%5D=/tmp/out')).toHaveLength(0);
    });

    it('does not linkify paths inside URL fragment or path-parameter values', () => {
      expect(findPaths('See https://host/page#file=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/download;file=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/page#artifact=/tmp/out.log')).toHaveLength(0);
      expect(findPaths('See https://host/download;output=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/page#path=/tmp/out&other=1')).toHaveLength(0);
    });

    it('does not linkify paths inside URL query values with hyphenated keys', () => {
      expect(findPaths('See https://host/download?file-name=/tmp/out')).toHaveLength(0);
      expect(findPaths('See https://host/download?content-type=/tmp/out.log')).toHaveLength(0);
      expect(findPaths('See https://host/download?x-custom-param=/tmp/out')).toHaveLength(0);
      expect(findPaths('Visit example.com?file-name=/tmp/out&other=1')).toHaveLength(0);
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

    it('detects paths with word parenthesized filename suffixes', () => {
      const matches = findPaths('Saved /Users/me/Downloads/report (final).pdf');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/Downloads/report (final).pdf');
    });

    it('detects paths with spaced or hyphenated parenthesized suffixes', () => {
      expect(findPaths('Saved /Users/me/Downloads/report (final copy).pdf')[0][1]).toBe(
        '/Users/me/Downloads/report (final copy).pdf'
      );
      expect(findPaths('Saved /tmp/report (final-draft).pdf')[0][1]).toBe(
        '/tmp/report (final-draft).pdf'
      );
    });

    it('detects paths with alphanumeric parenthesized filename suffixes', () => {
      const matches = findPaths('Saved /Users/me/Downloads/report (v2).pdf');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/Users/me/Downloads/report (v2).pdf');
    });

    it('detects paths with dotted version parenthesized suffixes', () => {
      expect(findPaths('Saved /tmp/report (v1.2).pdf')[0][1]).toBe('/tmp/report (v1.2).pdf');
      expect(findPaths('Saved /tmp/report (v1.2.3).pdf')[0][1]).toBe('/tmp/report (v1.2.3).pdf');
      expect(findPaths('Saved /tmp/report(v1.2).pdf')[0][1]).toBe('/tmp/report(v1.2).pdf');
    });

    it('detects paths with underscored parenthesized suffixes', () => {
      expect(findPaths('Saved /tmp/report (final_v2).pdf')[0][1]).toBe('/tmp/report (final_v2).pdf');
      expect(findPaths('Saved /tmp/report (draft_v2).pdf')[0][1]).toBe('/tmp/report (draft_v2).pdf');
      expect(findPaths('Saved /tmp/report(final_v2).pdf')[0][1]).toBe('/tmp/report(final_v2).pdf');
    });

    it('detects paths with spaced version parenthesized suffixes', () => {
      expect(findPaths('Saved /tmp/report (final copy v2).pdf')[0][1]).toBe(
        '/tmp/report (final copy v2).pdf'
      );
    });

    it('does not absorb dotted parenthetical prose without extension', () => {
      expect(findPaths('Created /tmp/out (temporary) for debugging')[0][1]).toBe('/tmp/out');
      expect(findPaths('Created /tmp/report (temp) for debugging')[0][1]).toBe('/tmp/report');
      expect(findPaths('Created /tmp/out(temporary) for debugging')[0][1]).toBe('/tmp/out');
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

    it('does not append prose after parenthesized filename suffixes', () => {
      const matches = findPaths('Saved /tmp/report (1) successfully.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/report (1)');
    });

    it('detects paths with commas in filenames', () => {
      const matches = findPaths('Saved /tmp/report,final.txt');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/report,final.txt');
    });

    it('detects paths with apostrophes in filenames', () => {
      const matches = findPaths("Saved /tmp/it's.txt");
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe("/tmp/it's.txt");
    });

    it('detects paths with apostrophes in parenthesized filename suffixes', () => {
      expect(findPaths("Saved /tmp/report (John's copy).pdf")[0][1]).toBe(
        "/tmp/report (John's copy).pdf"
      );
      expect(findPaths("Saved /tmp/report(John's).pdf")[0][1]).toBe("/tmp/report(John's).pdf");
    });

    it('detects paths with Unicode in parenthesized filename suffixes', () => {
      expect(findPaths('Saved /tmp/report (最終).pdf')[0][1]).toBe('/tmp/report (最終).pdf');
      expect(findPaths('Saved /tmp/report(最終).pdf')[0][1]).toBe('/tmp/report(最終).pdf');
      expect(findPaths('Saved /tmp/report (コピー v2).pdf')[0][1]).toBe(
        '/tmp/report (コピー v2).pdf'
      );
    });

    it('still rejects prose parentheticals without extension evidence', () => {
      expect(findPaths('Created /tmp/out (temporary) for debugging')[0][1]).toBe('/tmp/out');
      expect(findPaths('Created /tmp/report (temp) for debugging')[0][1]).toBe('/tmp/report');
      expect(findPaths('Created /tmp/out (temporary).')[0][1]).toBe('/tmp/out');
      expect(findPaths('Created /tmp/report (see notes) for review')[0][1]).toBe('/tmp/report');
    });

    it('detects paths with colons in timestamped filenames', () => {
      const matches = findPaths('Saved /tmp/2026-06-18T18:30:00.log');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/2026-06-18T18:30:00.log');
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

    it('does not extend paths with trailing numeric counts', () => {
      const matches = findPaths('Created /tmp/out 2 files');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/out');
    });

    it('does not extend paths with bare 4+ digit prose counts', () => {
      expect(findPaths('Created /tmp/out 2026 files')[0][1]).toBe('/tmp/out');
    });

    it('does not extend capitalized basenames with bare 4+ digit prose counts', () => {
      expect(findPaths('Created /tmp/Report 2026 files')[0][1]).toBe('/tmp/Report');
    });

    it('does not extend capitalized paths with short numeric counts', () => {
      expect(findPaths('Created /tmp/Out 2 files')[0][1]).toBe('/tmp/Out');
    });

    it('does not extend paths with capitalized prose', () => {
      const matches = findPaths('See /tmp/out Please review.');
      expect(matches).toHaveLength(1);
      expect(matches[0][1]).toBe('/tmp/out');
    });

    it('does not extend capitalized basenames with following prose', () => {
      expect(findPaths('See /tmp/Result Please review.')[0][1]).toBe('/tmp/Result');
    });

    it('detects paths with @ and percent-encoded characters in filenames', () => {
      expect(findPaths('File /tmp/foo@bar.txt')[0][1]).toBe('/tmp/foo@bar.txt');
      expect(findPaths('File /tmp/foo%20bar.txt')[0][1]).toBe('/tmp/foo%20bar.txt');
    });

    it('does not linkify partial paths before unsupported characters', () => {
      expect(findPaths('See /tmp/foo#bar.txt')).toHaveLength(0);
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

describe('file link trust', () => {
  it('trusts links whose label matches the decoded path', () => {
    const path = '/Users/me/My Project/file.txt';
    const href = OPEN_FILE_PROTOCOL + encodeURI(path);
    expect(isTrustedGeneratedFileLink(href, path)).toBe(true);
  });

  it('rejects authored links with mismatched labels', () => {
    const href = OPEN_FILE_PROTOCOL + encodeURI('/Users/me/Secrets');
    expect(isTrustedGeneratedFileLink(href, 'release notes')).toBe(false);
  });
});
