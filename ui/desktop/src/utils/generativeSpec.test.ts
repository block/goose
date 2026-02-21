import { describe, expect, it } from 'vitest';
import {
  extractGenerativeSpec,
  hasPartialGenerativeSpec,
  stripPartialGenerativeSpec,
} from './generativeSpec';

const GOOSE_UI_SPEC = JSON.stringify({
  root: 'main',
  elements: {
    main: { type: 'Card', children: ['title'] },
    title: { type: 'Text', props: { content: 'Hello' } },
  },
});

const JSONL_SPEC = [
  '{"op":"add","path":"/root","value":"main"}',
  '{"op":"add","path":"/elements/main","value":{"type":"Card","children":["title"]}}',
  '{"op":"add","path":"/elements/title","value":{"type":"Text","props":{"content":"Hello"}}}',
].join('\n');

describe('generativeSpec', () => {
  describe('extractGenerativeSpec', () => {
    it('extracts goose-ui fenced block', () => {
      const text = `Before text\n\`\`\`goose-ui\n${GOOSE_UI_SPEC}\n\`\`\`\nAfter text`;
      const result = extractGenerativeSpec(text);
      expect(result).not.toBeNull();
      expect(result?.spec.root).toBe('main');
      expect(result?.spec.elements).toHaveProperty('main');
      expect(result?.beforeText).toBe('Before text');
      expect(result?.afterText).toBe('After text');
    });

    it('extracts goose-ui XML tag', () => {
      const text = `Intro\n<goose-ui>${GOOSE_UI_SPEC}</goose-ui>\nOutro`;
      const result = extractGenerativeSpec(text);
      expect(result).not.toBeNull();
      expect(result?.spec.root).toBe('main');
      expect(result?.beforeText).toBe('Intro');
      expect(result?.afterText).toBe('Outro');
    });

    it('does NOT extract json-render fenced blocks (they use System 1 renderer)', () => {
      const text = `Here is the chart:\n\`\`\`json-render\n${JSONL_SPEC}\n\`\`\`\nAll done.`;
      const result = extractGenerativeSpec(text);
      expect(result).toBeNull();
    });

    it('does NOT extract jsonrender (no hyphen) fenced blocks', () => {
      const text = `\`\`\`jsonrender\n${JSONL_SPEC}\n\`\`\``;
      const result = extractGenerativeSpec(text);
      expect(result).toBeNull();
    });

    it('returns null for plain text', () => {
      expect(extractGenerativeSpec('Just some regular text')).toBeNull();
    });

    it('returns null for non-spec code block', () => {
      expect(extractGenerativeSpec('```javascript\nconsole.log("hi")\n```')).toBeNull();
    });
  });

  describe('hasPartialGenerativeSpec', () => {
    it('returns true for partial goose-ui block', () => {
      const text = 'Some text\n```goose-ui\n{"root": "main"';
      expect(hasPartialGenerativeSpec(text)).toBe(true);
    });

    it('returns true for partial json-render block', () => {
      const text = 'Some text\n```json-render\n{"op":"add","path":"/root"';
      expect(hasPartialGenerativeSpec(text)).toBe(true);
    });

    it('returns true for partial jsonrender block (no hyphen)', () => {
      const text = 'Building...\n```jsonrender\n{"op":"add"';
      expect(hasPartialGenerativeSpec(text)).toBe(true);
    });

    it('returns false for complete json-render block', () => {
      const text = `\`\`\`json-render\n${JSONL_SPEC}\n\`\`\``;
      expect(hasPartialGenerativeSpec(text)).toBe(false);
    });

    it('returns false for complete goose-ui block', () => {
      const text = `\`\`\`goose-ui\n${GOOSE_UI_SPEC}\n\`\`\``;
      expect(hasPartialGenerativeSpec(text)).toBe(false);
    });

    it('returns false for plain text', () => {
      expect(hasPartialGenerativeSpec('no spec here')).toBe(false);
    });
  });

  describe('stripPartialGenerativeSpec', () => {
    it('strips partial json-render block', () => {
      const text = 'Here is the UI:\n```json-render\n{"op":"add","path":"/root"';
      expect(stripPartialGenerativeSpec(text)).toBe('Here is the UI:');
    });

    it('strips partial jsonrender block (no hyphen)', () => {
      const text = 'Building...\n```jsonrender\n{"op":"add"';
      expect(stripPartialGenerativeSpec(text)).toBe('Building...');
    });

    it('strips partial goose-ui block', () => {
      const text = 'Before\n```goose-ui\n{"root":';
      expect(stripPartialGenerativeSpec(text)).toBe('Before');
    });

    it('does not strip complete json-render block', () => {
      const text = `Complete\n\`\`\`json-render\n${JSONL_SPEC}\n\`\`\`\nAfter`;
      expect(stripPartialGenerativeSpec(text)).toBe(text);
    });

    it('does not strip complete goose-ui block', () => {
      const text = `Complete\n\`\`\`goose-ui\n${GOOSE_UI_SPEC}\n\`\`\``;
      expect(stripPartialGenerativeSpec(text)).toBe(text);
    });
  });
});
