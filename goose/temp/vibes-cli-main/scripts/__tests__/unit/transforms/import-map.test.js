/**
 * Tests for import-map.js transform
 *
 * These functions manipulate import maps in HTML files.
 * Critical for React singleton correctness - bugs cause duplicate React instances.
 */

import { describe, it, expect } from 'vitest';
import {
  replaceImportMap,
  migrateDepsToExternal,
  addExternalParams,
  IMPORT_MAP_REGEX
} from '../../../lib/transforms/import-map.js';

describe('IMPORT_MAP_REGEX', () => {
  it('matches standard import map script tag', () => {
    const html = '<script type="importmap">{"imports":{}}</script>';
    expect(html.match(IMPORT_MAP_REGEX)).toBeTruthy();
  });

  it('matches with single quotes', () => {
    const html = "<script type='importmap'>{\"imports\":{}}</script>";
    expect(html.match(IMPORT_MAP_REGEX)).toBeTruthy();
  });

  it('captures content between tags', () => {
    const html = '<script type="importmap">{"imports":{"react":"url"}}</script>';
    const match = html.match(IMPORT_MAP_REGEX);
    expect(match[1]).toContain('react');
  });
});

describe('replaceImportMap', () => {
  it('replaces import map with new imports', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "react": "https://esm.sh/react@18.0.0"
  }
}
</script>`;
    const newImports = {
      react: 'https://esm.sh/react@19.2.1',
      'react-dom': 'https://esm.sh/react-dom@19.2.1'
    };
    const result = replaceImportMap(html, newImports);

    expect(result.success).toBe(true);
    expect(result.html).toContain('react@19.2.1');
    expect(result.html).toContain('react-dom@19.2.1');
    expect(result.html).not.toContain('react@18.0.0');
  });

  it('preserves scopes if present', () => {
    const html = `<script type="importmap">
{
  "imports": { "react": "https://esm.sh/react@18.0.0" },
  "scopes": { "/vendor/": { "lodash": "url" } }
}
</script>`;
    const newImports = { react: 'https://esm.sh/react@19.2.1' };
    const result = replaceImportMap(html, newImports);

    expect(result.success).toBe(true);
    expect(result.html).toContain('scopes');
    expect(result.html).toContain('/vendor/');
  });

  it('returns error when no import map found', () => {
    const html = '<html><body></body></html>';
    const result = replaceImportMap(html, { react: 'url' });

    expect(result.success).toBe(false);
    expect(result.error).toContain('No import map found');
  });

  it('returns error for malformed import map JSON', () => {
    const html = '<script type="importmap">{ invalid json }</script>';
    const result = replaceImportMap(html, { react: 'url' });

    expect(result.success).toBe(false);
    expect(result.error).toContain('Could not parse');
  });

  it('provides before/after diff', () => {
    const html = '<script type="importmap">{"imports":{"react":"old"}}</script>';
    const result = replaceImportMap(html, { react: 'new' });

    expect(result.success).toBe(true);
    expect(result.diff.before).toContain('old');
    expect(result.diff.after).toContain('new');
  });
});

describe('migrateDepsToExternal', () => {
  it('converts ?deps=react@X.Y.Z to ?external=react,react-dom', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "use-vibes": "https://esm.sh/use-vibes@0.19.0?deps=react@18.3.1"
  }
}
</script>`;
    const result = migrateDepsToExternal(html);

    expect(result.success).toBe(true);
    expect(result.html).toContain('?external=react,react-dom');
    expect(result.html).not.toContain('?deps=');
  });

  it('handles multiple packages with deps', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "use-vibes": "https://esm.sh/use-vibes@0.19.0?deps=react@18.0.0",
    "use-fireproof": "https://esm.sh/use-fireproof@0.20.0?deps=react@18.0.0"
  }
}
</script>`;
    const result = migrateDepsToExternal(html);

    expect(result.success).toBe(true);
    const matches = result.html.match(/\?external=react,react-dom/g);
    expect(matches.length).toBe(2);
  });

  it('returns error when no deps patterns found', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "react": "https://esm.sh/react@19.2.1"
  }
}
</script>`;
    const result = migrateDepsToExternal(html);

    expect(result.success).toBe(false);
    expect(result.error).toContain('No ?deps= patterns found');
  });

  it('returns error when no import map found', () => {
    const html = '<html></html>';
    const result = migrateDepsToExternal(html);

    expect(result.success).toBe(false);
    expect(result.error).toContain('No import map found');
  });
});

describe('addExternalParams', () => {
  it('adds ?external= to use-vibes without query params', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "use-vibes": "https://esm.sh/use-vibes@0.19.0"
  }
}
</script>`;
    const result = addExternalParams(html);

    expect(result.success).toBe(true);
    expect(result.html).toContain('use-vibes@0.19.0?external=react,react-dom');
  });

  it('adds ?external= to use-fireproof', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "use-fireproof": "https://esm.sh/use-fireproof@0.20.0"
  }
}
</script>`;
    const result = addExternalParams(html);

    expect(result.success).toBe(true);
    expect(result.html).toContain('use-fireproof@0.20.0?external=react,react-dom');
  });

  it('skips packages already with ?external=', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "use-vibes": "https://esm.sh/use-vibes@0.19.0?external=react,react-dom"
  }
}
</script>`;
    const result = addExternalParams(html);

    expect(result.success).toBe(false);
    expect(result.error).toContain('already have ?external=');
  });

  it('returns error when no import map found', () => {
    const html = '<html></html>';
    const result = addExternalParams(html);

    expect(result.success).toBe(false);
    expect(result.error).toContain('No import map found');
  });

  it('provides before/after diff', () => {
    const html = '<script type="importmap">{"imports":{"use-vibes":"https://esm.sh/use-vibes@0.19.0"}}</script>';
    const result = addExternalParams(html);

    expect(result.success).toBe(true);
    expect(result.diff).toBeDefined();
    expect(result.diff.after).toContain('external');
  });
});
