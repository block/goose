/**
 * Tests for analyze.js detection functions
 *
 * These functions analyze Vibes app HTML to detect template type,
 * era, versions, and patterns. Used by the update pipeline.
 */

import { describe, it, expect } from 'vitest';
import {
  extractImportMap,
  detectTemplateType,
  detectEra,
  extractLibraryVersions,
  detectQueryPatterns,
  findAppCodeBoundaries,
  ERAS
} from '../../lib/analyze.js';

describe('extractImportMap', () => {
  it('extracts import map from HTML', () => {
    const html = `<script type="importmap">
{
  "imports": {
    "react": "https://esm.sh/react@19.2.1"
  }
}
</script>`;
    const result = extractImportMap(html);
    expect(result).toEqual({
      imports: { react: 'https://esm.sh/react@19.2.1' }
    });
  });

  it('handles single quotes in type attribute', () => {
    const html = "<script type='importmap'>{\"imports\":{}}</script>";
    const result = extractImportMap(html);
    expect(result).toEqual({ imports: {} });
  });

  it('returns null when no import map found', () => {
    const html = '<html><body></body></html>';
    expect(extractImportMap(html)).toBeNull();
  });

  it('returns null for malformed JSON', () => {
    const html = '<script type="importmap">{ invalid }</script>';
    expect(extractImportMap(html)).toBeNull();
  });

  it('handles import map with scopes', () => {
    const html = `<script type="importmap">
{
  "imports": { "react": "url" },
  "scopes": { "/vendor/": { "lodash": "url" } }
}
</script>`;
    const result = extractImportMap(html);
    expect(result.imports).toBeDefined();
    expect(result.scopes).toBeDefined();
  });
});

describe('detectTemplateType', () => {
  describe('vibes-basic detection', () => {
    it('returns vibes-basic for simple app', () => {
      const html = '<html><body></body></html>';
      const importMap = { imports: { react: 'url' } };
      expect(detectTemplateType(html, importMap)).toBe('vibes-basic');
    });

    it('returns vibes-basic when no Clerk indicators', () => {
      const html = 'function App() { return <div>Hello</div>; }';
      const importMap = { imports: { 'use-vibes': 'url' } };
      expect(detectTemplateType(html, importMap)).toBe('vibes-basic');
    });
  });

  describe('sell template detection', () => {
    it('detects sell by Clerk import', () => {
      const html = '<html></html>';
      const importMap = {
        imports: {
          '@clerk/clerk-react': 'https://esm.sh/@clerk/clerk-react@4.0.0'
        }
      };
      expect(detectTemplateType(html, importMap)).toBe('sell');
    });

    it('detects sell by ClerkProvider component', () => {
      const html = '<ClerkProvider publishableKey={key}></ClerkProvider>';
      const importMap = { imports: {} };
      expect(detectTemplateType(html, importMap)).toBe('sell');
    });

    it('detects sell by useAuth hook', () => {
      const html = 'const { userId } = useAuth();';
      const importMap = { imports: {} };
      expect(detectTemplateType(html, importMap)).toBe('sell');
    });

    it('detects sell by useUser hook', () => {
      const html = 'const { user } = useUser();';
      const importMap = { imports: {} };
      expect(detectTemplateType(html, importMap)).toBe('sell');
    });

    it('detects sell by subdomain routing', () => {
      const html = 'const hostname = window.location.hostname; const subdomain = hostname.split(".")[0];';
      const importMap = { imports: {} };
      expect(detectTemplateType(html, importMap)).toBe('sell');
    });

    it('detects sell by CLERK_PUBLISHABLE_KEY placeholder', () => {
      const html = 'const key = "__CLERK_PUBLISHABLE_KEY__";';
      const importMap = { imports: {} };
      expect(detectTemplateType(html, importMap)).toBe('sell');
    });
  });
});

describe('detectEra', () => {
  it('returns sell-v1 for sell template', () => {
    const imports = { 'use-vibes': 'url' };
    expect(detectEra(imports, 'sell')).toBe('sell-v1');
  });

  it('returns 0.19.x-dev for dev versions', () => {
    const imports = { 'use-vibes': 'https://esm.sh/use-vibes@0.19.0-dev' };
    expect(detectEra(imports, 'vibes-basic')).toBe('0.19.x-dev');
  });

  it('returns 0.19.x-dev for preview versions', () => {
    const imports = { 'use-vibes': 'https://esm.sh/use-vibes@0.19.0-dev-preview-50' };
    expect(detectEra(imports, 'vibes-basic')).toBe('0.19.x-dev');
  });

  it('returns 0.18.x for external pattern', () => {
    const imports = { 'use-vibes': 'https://esm.sh/use-vibes@0.18.9?external=react,react-dom' };
    expect(detectEra(imports, 'vibes-basic')).toBe('0.18.x');
  });

  it('returns sell-v1 for deps pattern', () => {
    const imports = { 'use-vibes': 'https://esm.sh/use-vibes@0.19.0?deps=react@18.3.1' };
    expect(detectEra(imports, 'vibes-basic')).toBe('sell-v1');
  });

  it('returns pre-0.18 for URLs without query params', () => {
    const imports = { 'use-vibes': 'https://esm.sh/use-vibes@0.17.0' };
    expect(detectEra(imports, 'vibes-basic')).toBe('pre-0.18');
  });

  it('returns unknown for null imports', () => {
    expect(detectEra(null, 'vibes-basic')).toBe('unknown');
  });
});

describe('extractLibraryVersions', () => {
  it('extracts all library versions', () => {
    const imports = {
      'react': 'https://esm.sh/react@19.2.1',
      'react-dom': 'https://esm.sh/react-dom@19.2.1',
      'use-vibes': 'https://esm.sh/use-vibes@0.18.9?external=react',
      'use-fireproof': 'https://esm.sh/use-fireproof@0.20.0',
      '@clerk/clerk-react': 'https://esm.sh/@clerk/clerk-react@4.30.0'
    };
    const result = extractLibraryVersions(imports);

    expect(result.react).toBe('19.2.1');
    expect(result.reactDom).toBe('19.2.1');
    expect(result.useVibes).toBe('0.18.9');
    expect(result.useFireproof).toBe('0.20.0');
    expect(result.clerk).toBe('4.30.0');
  });

  it('handles missing libraries gracefully', () => {
    const imports = { react: 'https://esm.sh/react@19.2.1' };
    const result = extractLibraryVersions(imports);

    expect(result.react).toBe('19.2.1');
    expect(result.useVibes).toBeNull();
    expect(result.clerk).toBeNull();
  });

  it('returns empty object for null imports', () => {
    expect(extractLibraryVersions(null)).toEqual({});
  });
});

describe('detectQueryPatterns', () => {
  it('detects ?external= pattern', () => {
    const imports = {
      'use-vibes': 'https://esm.sh/use-vibes@0.18.9?external=react,react-dom'
    };
    const result = detectQueryPatterns(imports);

    expect(result.usesExternal).toBe(true);
    expect(result.usesDeps).toBe(false);
  });

  it('detects ?deps= pattern', () => {
    const imports = {
      'use-vibes': 'https://esm.sh/use-vibes@0.19.0?deps=react@18.3.1'
    };
    const result = detectQueryPatterns(imports);

    expect(result.usesExternal).toBe(false);
    expect(result.usesDeps).toBe(true);
  });

  it('detects React 18 pinning', () => {
    const imports = {
      react: 'https://esm.sh/react@18.3.1',
      'use-vibes': 'https://esm.sh/use-vibes@0.19.0'
    };
    const result = detectQueryPatterns(imports);

    expect(result.reactPinned).toBe(true);
  });

  it('returns false flags for null imports', () => {
    const result = detectQueryPatterns(null);

    expect(result.usesExternal).toBe(false);
    expect(result.usesDeps).toBe(false);
  });
});

describe('findAppCodeBoundaries', () => {
  it('finds placeholder marker', () => {
    const html = `
<script>
// __VIBES_APP_CODE__
function App() {}
</script>`;
    const result = findAppCodeBoundaries(html);

    expect(result.type).toBe('placeholder');
    expect(result.marker).toBe('// __VIBES_APP_CODE__');
  });

  it('finds babel module script block', () => {
    const html = `
<script type="text/babel" data-type="module">
function App() { return <div>Hello</div>; }
</script>`;
    const result = findAppCodeBoundaries(html);

    expect(result.type).toBe('babel-module');
    expect(result.content).toContain('function App');
  });

  it('returns null when no app code found', () => {
    const html = '<html><body></body></html>';
    expect(findAppCodeBoundaries(html)).toBeNull();
  });
});

describe('ERAS constant', () => {
  it('defines expected eras', () => {
    expect(ERAS).toHaveProperty('pre-0.18');
    expect(ERAS).toHaveProperty('0.18.x');
    expect(ERAS).toHaveProperty('0.19.x-dev');
    expect(ERAS).toHaveProperty('sell-v1');
  });

  it('each era has required properties', () => {
    for (const [key, era] of Object.entries(ERAS)) {
      expect(era.name).toBeDefined();
      expect(era.markers).toBeDefined();
      expect(Array.isArray(era.markers)).toBe(true);
      expect(era.notes).toBeDefined();
    }
  });
});
