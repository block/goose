/**
 * Tests for template merging utilities
 *
 * These tests ensure merge-templates.js logic correctly:
 * - Replaces placeholders with content
 * - Validates template structure
 */

import { describe, it, expect } from 'vitest';
import {
  mergeTemplate,
  validateBasePlaceholders
} from '../../lib/template-merge.js';

describe('mergeTemplate', () => {
  const baseTemplate = `<!DOCTYPE html>
<html>
<head>
  <title>__TITLE__</title>
</head>
<body>
  <script>
    // === COMPONENTS_PLACEHOLDER ===
  </script>
  <!-- === DELTA_PLACEHOLDER === -->
</body>
</html>`;

  const components = `// Built components
function VibesSwitch() { return null; }
window.VibesSwitch = VibesSwitch;`;

  const delta = `<script>
  function App() { return "Hello"; }
</script>`;

  describe('title replacement', () => {
    it('replaces __TITLE__ with skill title', () => {
      const skill = { name: 'vibes', title: 'My App Title' };
      const result = mergeTemplate(skill, baseTemplate, components, delta);
      expect(result).toContain('<title>My App Title</title>');
      expect(result).not.toContain('__TITLE__');
    });

    it('handles dynamic title placeholder', () => {
      const skill = { name: 'sell', title: '__APP_TITLE__' };
      const result = mergeTemplate(skill, baseTemplate, components, delta);
      expect(result).toContain('<title>__APP_TITLE__</title>');
    });
  });

  describe('components injection', () => {
    it('replaces components placeholder with component code', () => {
      const skill = { name: 'vibes', title: 'Test' };
      const result = mergeTemplate(skill, baseTemplate, components, delta);
      expect(result).toContain('function VibesSwitch()');
      expect(result).toContain('window.VibesSwitch');
      expect(result).not.toContain('// === COMPONENTS_PLACEHOLDER ===');
    });

    it('preserves component code structure', () => {
      const skill = { name: 'vibes', title: 'Test' };
      const multiLineComponents = `// Line 1
function A() {}
// Line 2
function B() {}`;
      const result = mergeTemplate(skill, baseTemplate, multiLineComponents, delta);
      expect(result).toContain('// Line 1');
      expect(result).toContain('// Line 2');
    });
  });

  describe('delta injection', () => {
    it('replaces delta placeholder with delta content', () => {
      const skill = { name: 'vibes', title: 'Test' };
      const result = mergeTemplate(skill, baseTemplate, components, delta);
      expect(result).toContain('function App()');
      expect(result).not.toContain('<!-- === DELTA_PLACEHOLDER === -->');
    });

    it('handles multi-line delta content', () => {
      const skill = { name: 'vibes', title: 'Test' };
      const multiLineDelta = `<script>
  // Comment
  function App() {
    return "Multi-line";
  }
</script>`;
      const result = mergeTemplate(skill, baseTemplate, components, multiLineDelta);
      expect(result).toContain('// Comment');
      expect(result).toContain('return "Multi-line"');
    });
  });

  describe('full merge', () => {
    it('produces valid merged template', () => {
      const skill = { name: 'vibes', title: 'Made on Vibes DIY' };
      const result = mergeTemplate(skill, baseTemplate, components, delta);

      // All replacements done
      expect(result).not.toContain('__TITLE__');
      expect(result).not.toContain('COMPONENTS_PLACEHOLDER');
      expect(result).not.toContain('DELTA_PLACEHOLDER');

      // All content present
      expect(result).toContain('Made on Vibes DIY');
      expect(result).toContain('VibesSwitch');
      expect(result).toContain('function App()');

      // HTML structure preserved
      expect(result).toContain('<!DOCTYPE html>');
      expect(result).toContain('</html>');
    });
  });
});

describe('validateBasePlaceholders', () => {
  describe('valid templates', () => {
    it('returns valid for template with all placeholders', () => {
      const template = `<!DOCTYPE html>
<title>__TITLE__</title>
// === COMPONENTS_PLACEHOLDER ===
<!-- === DELTA_PLACEHOLDER === -->`;
      const result = validateBasePlaceholders(template);
      expect(result.valid).toBe(true);
      expect(result.missing).toEqual([]);
    });
  });

  describe('invalid templates', () => {
    it('detects missing __TITLE__', () => {
      const template = `<!DOCTYPE html>
// === COMPONENTS_PLACEHOLDER ===
<!-- === DELTA_PLACEHOLDER === -->`;
      const result = validateBasePlaceholders(template);
      expect(result.valid).toBe(false);
      expect(result.missing).toContain('__TITLE__');
    });

    it('detects missing COMPONENTS_PLACEHOLDER', () => {
      const template = `<!DOCTYPE html>
<title>__TITLE__</title>
<!-- === DELTA_PLACEHOLDER === -->`;
      const result = validateBasePlaceholders(template);
      expect(result.valid).toBe(false);
      expect(result.missing).toContain('// === COMPONENTS_PLACEHOLDER ===');
    });

    it('detects missing DELTA_PLACEHOLDER', () => {
      const template = `<!DOCTYPE html>
<title>__TITLE__</title>
// === COMPONENTS_PLACEHOLDER ===`;
      const result = validateBasePlaceholders(template);
      expect(result.valid).toBe(false);
      expect(result.missing).toContain('<!-- === DELTA_PLACEHOLDER === -->');
    });

    it('detects multiple missing placeholders', () => {
      const template = '<!DOCTYPE html>';
      const result = validateBasePlaceholders(template);
      expect(result.valid).toBe(false);
      expect(result.missing).toHaveLength(3);
    });
  });

  describe('empty template', () => {
    it('reports all placeholders as missing', () => {
      const result = validateBasePlaceholders('');
      expect(result.valid).toBe(false);
      expect(result.missing).toHaveLength(3);
    });
  });
});
