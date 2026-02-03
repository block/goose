/**
 * Unit tests for assembly validation logic
 *
 * Tests the validation functions that catch errors before assembly completes.
 * These mirror the logic in assemble.js and assemble-sell.js.
 */

import { describe, it, expect } from 'vitest';

// ============== Validation Logic (extracted from assemble.js) ==============

const PLACEHOLDER = '// __VIBES_APP_CODE__';

/**
 * Validate assembled output
 * Mirrors the logic in assemble.js
 */
function validateAssembly(html, code) {
  const errors = [];

  if (!code || code.trim().length === 0) {
    errors.push('App code is empty');
  }

  if (html.includes(PLACEHOLDER)) {
    errors.push('Placeholder was not replaced');
  }

  if (!html.includes('export default function') && !html.includes('function App')) {
    errors.push('No App component found');
  }

  const scriptOpens = (html.match(/<script/gi) || []).length;
  const scriptCloses = (html.match(/<\/script>/gi) || []).length;
  if (scriptOpens !== scriptCloses) {
    errors.push(`Mismatched script tags: ${scriptOpens} opens, ${scriptCloses} closes`);
  }

  return errors;
}

/**
 * Validate sell assembly output
 * Mirrors the logic in assemble-sell.js
 */
function validateSellAssembly(html, app, admin) {
  const errors = [];

  if (!app || app.trim().length === 0) {
    errors.push('App code is empty');
  }

  if (!admin || admin.trim().length === 0) {
    errors.push('Admin code is empty');
  }

  const unreplaced = html.match(/__[A-Z_]+__/g) || [];
  if (unreplaced.length > 0) {
    errors.push(`Unreplaced placeholders: ${[...new Set(unreplaced)].join(', ')}`);
  }

  if (!html.includes('export default function') && !html.includes('function App')) {
    errors.push('No App component found');
  }

  return errors;
}

// ============== Tests ==============

describe('validateAssembly', () => {
  it('returns no errors for valid assembly', () => {
    const html = '<script>export default function App() { return <div>Hello</div>; }</script>';
    const code = 'export default function App() { return <div>Hello</div>; }';
    expect(validateAssembly(html, code)).toEqual([]);
  });

  it('detects empty app code', () => {
    const html = '<script></script>';
    const errors = validateAssembly(html, '');
    expect(errors).toContain('App code is empty');
  });

  it('detects whitespace-only app code', () => {
    const html = '<script>   </script>';
    const errors = validateAssembly(html, '   \n\t  ');
    expect(errors).toContain('App code is empty');
  });

  it('detects unreplaced placeholder', () => {
    const html = `<script>${PLACEHOLDER}</script>`;
    const errors = validateAssembly(html, 'some code');
    expect(errors).toContain('Placeholder was not replaced');
  });

  it('detects missing App component - no function', () => {
    const html = '<script>const x = 1;</script>';
    const errors = validateAssembly(html, 'const x = 1;');
    expect(errors).toContain('No App component found');
  });

  it('accepts function App syntax', () => {
    const html = '<script>function App() { return null; }</script>';
    const code = 'function App() { return null; }';
    expect(validateAssembly(html, code)).toEqual([]);
  });

  it('accepts export default function syntax', () => {
    const html = '<script>export default function MyComponent() { return null; }</script>';
    const code = 'export default function MyComponent() { return null; }';
    expect(validateAssembly(html, code)).toEqual([]);
  });

  it('detects mismatched script tags - more opens', () => {
    const html = '<script><script>export default function App() {}</script>';
    const errors = validateAssembly(html, 'export default function App() {}');
    expect(errors.some(e => e.includes('Mismatched script tags'))).toBe(true);
  });

  it('detects mismatched script tags - more closes', () => {
    const html = '<script>export default function App() {}</script></script>';
    const errors = validateAssembly(html, 'export default function App() {}');
    expect(errors.some(e => e.includes('Mismatched script tags'))).toBe(true);
  });

  it('reports multiple errors at once', () => {
    const html = `<script>${PLACEHOLDER}</script></script>`;
    const errors = validateAssembly(html, '');
    expect(errors.length).toBeGreaterThanOrEqual(2);
    expect(errors).toContain('App code is empty');
  });
});

describe('validateSellAssembly', () => {
  it('returns no errors for valid sell assembly', () => {
    const html = '<script>export default function App() { return null; }</script>';
    const app = 'export default function App() {}';
    const admin = 'function AdminDashboard() {}';
    expect(validateSellAssembly(html, app, admin)).toEqual([]);
  });

  it('detects empty app code', () => {
    const errors = validateSellAssembly('<html></html>', '', 'admin code');
    expect(errors).toContain('App code is empty');
  });

  it('detects empty admin code', () => {
    const errors = validateSellAssembly('<html></html>', 'app code', '');
    expect(errors).toContain('Admin code is empty');
  });

  it('detects unreplaced placeholders', () => {
    const html = '<html>__CLERK_KEY__ and __APP_NAME__</html>';
    const errors = validateSellAssembly(html, 'app', 'admin');
    expect(errors.some(e => e.includes('Unreplaced placeholders'))).toBe(true);
    expect(errors.some(e => e.includes('__CLERK_KEY__'))).toBe(true);
  });

  it('deduplicates repeated placeholders', () => {
    const html = '<html>__TEST__ __TEST__ __TEST__</html>';
    const errors = validateSellAssembly(html, 'app', 'admin');
    const placeholderError = errors.find(e => e.includes('Unreplaced'));
    // Should only list __TEST__ once
    expect(placeholderError.match(/__TEST__/g).length).toBe(1);
  });

  it('reports multiple errors at once', () => {
    const html = '<html>__PLACEHOLDER__</html>';
    const errors = validateSellAssembly(html, '', '');
    expect(errors).toContain('App code is empty');
    expect(errors).toContain('Admin code is empty');
    expect(errors.some(e => e.includes('Unreplaced'))).toBe(true);
  });
});
