/**
 * Utilities for detecting and extracting generative UI specs from message text.
 *
 * The LLM can embed JSON UI specs in messages using either:
 *   1. A fenced code block with language `goose-ui`
 *   2. An XML-style <goose-ui>...</goose-ui> tag
 *   3. A fenced code block with language `json-render` (JSONL streaming format)
 *
 * Formats 1 & 2 are extracted here and rendered via GooseGenerativeUI (System 2,
 * 23 custom components: StatCard, DataCard, TabBar, etc.).
 *
 * Format 3 (json-render) is NOT extracted here — it flows through
 * MarkdownCode → JsonRenderBlock → CatalogRenderer (System 1, 33 shadcn
 * components including Heading, Tabs, Dialog, etc.). We only detect/strip
 * partial json-render blocks during streaming to prevent raw JSON from showing.
 */

import type { Spec } from '@json-render/react';
import { isGooseUISpec } from '../components/ui/design-system/goose-renderer';
import { looksLikeJsonRenderSpec } from './jsonRenderDetection';

interface ExtractedSpec {
  spec: Spec;
  beforeText: string;
  afterText: string;
}

const FENCED_BLOCK_RE = /```goose-ui\s*\n([\s\S]*?)```/;
const XML_TAG_RE = /<goose-ui>([\s\S]*?)<\/goose-ui>/;
const FENCED_JSONRENDER_RE = /```json-?render\s*\n([\s\S]*?)```/;

// Streaming-aware: detect incomplete specs that are still being typed
const PARTIAL_FENCED_RE = /```goose-ui\s*\n[\s\S]*$/;
const PARTIAL_XML_RE = /<goose-ui>[\s\S]*$/;
const PARTIAL_JSONRENDER_RE = /```json-?render\s*\n[\s\S]*$/;

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
 *
 * Only extracts goose-ui specs (formats 1 & 2). json-render blocks (format 3)
 * are intentionally left in the text so they render via MarkdownCode →
 * JsonRenderBlock → CatalogRenderer, which has the full 33-component registry.
 */
export function extractGenerativeSpec(text: string): ExtractedSpec | null {
  // Try fenced code block first: ```goose-ui ... ```
  const fencedMatch = text.match(FENCED_BLOCK_RE);
  if (fencedMatch) {
    const spec = tryParseSpec(fencedMatch[1]);
    if (spec) {
      const idx = fencedMatch.index ?? 0;
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
      const idx = xmlMatch.index ?? 0;
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
 * Covers all 3 formats to prevent raw JSON from showing.
 */
export function hasPartialGenerativeSpec(text: string): boolean {
  // If the content is a complete json-render spec (jsonl or json object), don't treat it as partial.
  if (looksLikeJsonRenderSpec(text)) {
    return false;
  }
  // If we already have a complete spec, it's not partial
  if (FENCED_BLOCK_RE.test(text) || XML_TAG_RE.test(text) || FENCED_JSONRENDER_RE.test(text)) {
    return false;
  }
  return (
    PARTIAL_FENCED_RE.test(text) || PARTIAL_XML_RE.test(text) || PARTIAL_JSONRENDER_RE.test(text)
  );
}

/**
 * Strip incomplete generative spec markup from streaming text
 * so it doesn't show raw JSON to the user.
 * Covers all 3 formats.
 */
export function stripPartialGenerativeSpec(text: string): string {
  // Only strip if partial (not complete)
  if (FENCED_BLOCK_RE.test(text) || XML_TAG_RE.test(text) || FENCED_JSONRENDER_RE.test(text)) {
    return text;
  }
  return text
    .replace(/```goose-ui\s*\n[\s\S]*$/, '')
    .replace(/<goose-ui>[\s\S]*$/, '')
    .replace(/```json-?render\s*\n[\s\S]*$/, '')
    .trim();
}
