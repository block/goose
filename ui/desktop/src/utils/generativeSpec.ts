/**
 * Utilities for detecting and extracting generative UI specs from message text.
 *
 * The LLM can embed JSON UI specs in messages using either:
 *   1. A fenced code block with language `goose-ui`
 *   2. An XML-style <goose-ui>...</goose-ui> tag
 *
 * Both formats wrap a JSON object matching the json-render Spec shape:
 *   { "root": "...", "elements": { ... } }
 */

import type { Spec } from '@json-render/react';
import { isGooseUISpec } from '../components/ui/design-system/goose-renderer';

interface ExtractedSpec {
  spec: Spec;
  beforeText: string;
  afterText: string;
}

const FENCED_BLOCK_RE = /```goose-ui\s*\n([\s\S]*?)```/;
const XML_TAG_RE = /<goose-ui>([\s\S]*?)<\/goose-ui>/;

// Streaming-aware: detect incomplete specs that are still being typed
const PARTIAL_FENCED_RE = /```goose-ui\s*\n[\s\S]*$/;
const PARTIAL_XML_RE = /<goose-ui>[\s\S]*$/;

function tryParseSpec(raw: string): Spec | null {
  try {
    const parsed = JSON.parse(raw.trim());
    if (isGooseUISpec(parsed)) {
      return parsed as Spec;
    }
  } catch {
    // not valid JSON yet
  }
  return null;
}

/**
 * Extract a generative UI spec from message text.
 * Returns the spec and surrounding text, or null if no spec found.
 */
export function extractGenerativeSpec(text: string): ExtractedSpec | null {
  // Try fenced code block first: ```goose-ui ... ```
  const fencedMatch = text.match(FENCED_BLOCK_RE);
  if (fencedMatch) {
    const spec = tryParseSpec(fencedMatch[1]);
    if (spec) {
      const idx = fencedMatch.index!;
      return {
        spec,
        beforeText: text.slice(0, idx).trim(),
        afterText: text.slice(idx + fencedMatch[0].length).trim(),
      };
    }
  }

  // Try XML tag: <goose-ui>...</goose-ui>
  const xmlMatch = text.match(XML_TAG_RE);
  if (xmlMatch) {
    const spec = tryParseSpec(xmlMatch[1]);
    if (spec) {
      const idx = xmlMatch.index!;
      return {
        spec,
        beforeText: text.slice(0, idx).trim(),
        afterText: text.slice(idx + xmlMatch[0].length).trim(),
      };
    }
  }

  return null;
}

/**
 * Check if text contains a partial (still-streaming) generative spec.
 * Used to suppress rendering incomplete specs during streaming.
 */
export function hasPartialGenerativeSpec(text: string): boolean {
  // If we already have a complete spec, it's not partial
  if (FENCED_BLOCK_RE.test(text) || XML_TAG_RE.test(text)) {
    return false;
  }
  return PARTIAL_FENCED_RE.test(text) || PARTIAL_XML_RE.test(text);
}

/**
 * Strip incomplete generative spec markup from streaming text
 * so it doesn't show raw JSON to the user.
 */
export function stripPartialGenerativeSpec(text: string): string {
  // Only strip if partial (not complete)
  if (FENCED_BLOCK_RE.test(text) || XML_TAG_RE.test(text)) {
    return text;
  }
  return text
    .replace(/```goose-ui\s*\n[\s\S]*$/, '')
    .replace(/<goose-ui>[\s\S]*$/, '')
    .trim();
}
