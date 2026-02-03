/**
 * Tests for import map and style prompt parsing
 *
 * These tests ensure the parsing logic handles various upstream file formats
 * and edge cases correctly.
 */

import { describe, it, expect } from 'vitest';
import { parseImportMapTs, parseStylePromptsTs, REQUIRED_IMPORT_MAP_KEYS } from '../../lib/parsers.js';

describe('parseImportMapTs', () => {
  describe('static string values', () => {
    it('parses quoted keys with static URLs', () => {
      const input = `{
        "react": "https://esm.sh/react@19.2.1",
        "react-dom": "https://esm.sh/react-dom@19.2.1"
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).toHaveProperty('react', 'https://esm.sh/react@19.2.1');
      expect(result).toHaveProperty('react-dom', 'https://esm.sh/react-dom@19.2.1');
    });

    it('parses unquoted keys with static URLs', () => {
      const input = `{
        react: "https://esm.sh/react@19.2.1",
        reactDom: "https://esm.sh/react-dom@19.2.1"
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).toHaveProperty('react', 'https://esm.sh/react@19.2.1');
      expect(result).toHaveProperty('reactDom', 'https://esm.sh/react-dom@19.2.1');
    });

    it('handles hyphenated package names', () => {
      const input = `{
        "react-dom": "https://esm.sh/react-dom@19.2.1",
        "use-fireproof": "https://esm.sh/use-fireproof@1.0.0"
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).toHaveProperty('react-dom');
      expect(result).toHaveProperty('use-fireproof');
    });
  });

  describe('template literals with VIBES_VERSION', () => {
    it('resolves VIBES_VERSION in template literals', () => {
      const input = `
        const VIBES_VERSION = "0.19";
        export const imports = {
          "use-vibes": \`https://esm.sh/use-vibes@\${VIBES_VERSION}\`,
        }
      `;
      const result = parseImportMapTs(input, { silent: true });
      expect(result['use-vibes']).toBe('https://esm.sh/use-vibes@0.19');
    });

    it('uses default version 0.19 when VIBES_VERSION not found', () => {
      const input = `{
        "use-vibes": \`https://esm.sh/use-vibes@\${VIBES_VERSION}\`
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result['use-vibes']).toBe('https://esm.sh/use-vibes@0.19');
    });

    it('handles mixed static and template values', () => {
      const input = `
        const VIBES_VERSION = "0.24.3-dev";
        export const imports = {
          "react": "https://esm.sh/react@19.2.1",
          "use-vibes": \`https://esm.sh/use-vibes@\${VIBES_VERSION}\`
        }
      `;
      const result = parseImportMapTs(input, { silent: true });
      expect(result['react']).toBe('https://esm.sh/react@19.2.1');
      expect(result['use-vibes']).toBe('https://esm.sh/use-vibes@0.24.3-dev');
    });
  });

  describe('edge cases', () => {
    it('ignores non-URL values', () => {
      const input = `{
        "name": "my-package",
        "version": "1.0.0",
        "react": "https://esm.sh/react@19.2.1"
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).not.toHaveProperty('name');
      expect(result).not.toHaveProperty('version');
      expect(result).toHaveProperty('react');
    });

    it('handles extra whitespace', () => {
      const input = `{
        "react"  :   "https://esm.sh/react@19.2.1"  ,
        react   :    "https://esm.sh/react@19.2.2"
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).toHaveProperty('react');
    });

    it('handles single quotes', () => {
      const input = `{
        'react': 'https://esm.sh/react@19.2.1'
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).toHaveProperty('react', 'https://esm.sh/react@19.2.1');
    });

    it('returns empty object for empty input', () => {
      const result = parseImportMapTs('', { silent: true });
      expect(result).toEqual({});
    });

    it('returns empty object for input with no valid imports', () => {
      const input = `{
        "name": "not-a-url",
        "count": 42
      }`;
      const result = parseImportMapTs(input, { silent: true });
      expect(result).toEqual({});
    });
  });

  describe('validation', () => {
    it('exports REQUIRED_IMPORT_MAP_KEYS constant', () => {
      expect(REQUIRED_IMPORT_MAP_KEYS).toContain('react');
      expect(REQUIRED_IMPORT_MAP_KEYS).toContain('react-dom');
    });
  });
});

describe('parseStylePromptsTs', () => {
  describe('default style extraction', () => {
    it('extracts prompt for DEFAULT_STYLE_NAME', () => {
      const input = `
        const DEFAULT_STYLE_NAME = "brutalist web";
        export const stylePrompts = [
          { name: "brutalist web", prompt: "Create a brutalist UI theme" }
        ];
      `;
      const result = parseStylePromptsTs(input);
      expect(result).toBe('Create a brutalist UI theme');
    });

    it('handles escaped characters in prompt', () => {
      const input = `
        const DEFAULT_STYLE_NAME = "brutalist web";
        export const stylePrompts = [
          { name: "brutalist web", prompt: "Line 1\\nLine 2" }
        ];
      `;
      const result = parseStylePromptsTs(input);
      expect(result).toBe('Line 1\nLine 2');
    });
  });

  describe('fallback behavior', () => {
    it('falls back to brutalist-style prompt when default not found', () => {
      const input = `
        export const stylePrompts = [
          { name: "other", prompt: "Create a UI theme in a neo-brutalist style with bold shapes" }
        ];
      `;
      const result = parseStylePromptsTs(input);
      expect(result).toContain('neo-brutalist style');
    });

    it('returns empty string when no matching prompt found', () => {
      const input = `
        export const stylePrompts = [
          { name: "other", prompt: "Something else entirely" }
        ];
      `;
      const result = parseStylePromptsTs(input);
      expect(result).toBe('');
    });
  });

  describe('edge cases', () => {
    it('returns empty string for empty input', () => {
      const result = parseStylePromptsTs('');
      expect(result).toBe('');
    });

    it('handles backtick-quoted prompts', () => {
      const input = `
        const DEFAULT_STYLE_NAME = "test";
        export const stylePrompts = [
          { name: "test", prompt: \`Multi-line
          prompt content\` }
        ];
      `;
      const result = parseStylePromptsTs(input);
      expect(result).toContain('Multi-line');
    });
  });
});
