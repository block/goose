import { createSpecStreamCompiler, nestedToFlat } from '@json-render/core';
import React, { useMemo } from 'react';
import { CatalogRenderer, GOOSE_CUSTOM_COMPONENT_KEYS, SHADCN_COMPONENT_KEYS } from './setup';

interface JsonRenderBlockProps {
  spec: string;
}

interface Spec {
  root?: string;
  elements?: Record<string, unknown>;
  state?: Record<string, unknown>;
}

type UnknownComponentType = { elementId: string; type: string };

const MAX_SPEC_CHARS = 250_000;
const MAX_JSONL_LINES = 5_000;
const MAX_ELEMENTS = 5_000;

function getUnknownComponentTypes(elements: Record<string, unknown>): UnknownComponentType[] {
  const known = new Set<string>([...SHADCN_COMPONENT_KEYS, ...GOOSE_CUSTOM_COMPONENT_KEYS]);

  const unknown: UnknownComponentType[] = [];
  for (const [elementId, el] of Object.entries(elements)) {
    if (!el || typeof el !== 'object') continue;
    const t = (el as { type?: unknown }).type;
    if (typeof t !== 'string') continue;
    if (!known.has(t)) {
      unknown.push({ elementId, type: t });
    }
  }

  return unknown;
}

/**
 * Detect whether the spec string is JSONL (streaming patches) or nested JSON tree.
 * JSONL: each line is {"op":"add","path":"/...","value":...}
 * Nested JSON: a single JSON object with "root" as an object (not a string)
 */
function isJsonlFormat(text: string): boolean {
  const firstLine = text.trim().split('\n')[0];
  if (!firstLine) return false;
  try {
    const parsed = JSON.parse(firstLine);
    return parsed.op === 'add' && typeof parsed.path === 'string';
  } catch {
    return false;
  }
}

/**
 * Recover a malformed JSONL line by stripping extra trailing braces.
 * LLMs sometimes produce an extra } on deeply nested objects.
 */
function recoverJsonLine(line: string): string {
  const trimmed = line.trim();
  if (!trimmed.startsWith('{')) return trimmed;
  try {
    JSON.parse(trimmed);
    return trimmed;
  } catch {
    let attempt = trimmed;
    while (attempt.length > 2 && attempt.endsWith('}')) {
      attempt = attempt.slice(0, -1);
      try {
        JSON.parse(attempt);
        return attempt;
      } catch {}
    }
    return trimmed;
  }
}

/**
 * Parse a JSONL streaming spec into the flat Spec format using createSpecStreamCompiler.
 * Pre-processes lines to recover from common LLM JSON errors (extra trailing braces).
 */
function parseJsonlSpec(text: string): {
  spec: Spec;
  recoveredLineCount: number;
  lineCount: number;
} {
  let recoveredLineCount = 0;
  const lines = text.split('\n');
  const recovered = lines
    .map((line) => {
      const fixed = recoverJsonLine(line);
      if (fixed !== line.trim()) {
        recoveredLineCount += 1;
      }
      return fixed;
    })
    .join('\n');

  const compiler = createSpecStreamCompiler<Spec>();
  compiler.push(`${recovered}\n`);
  return { spec: compiler.getResult(), recoveredLineCount, lineCount: lines.length };
}

/**
 * Parse a nested JSON tree spec and convert to flat Spec format.
 */
function parseNestedSpec(text: string): Spec | null {
  try {
    const raw = JSON.parse(text);
    const rootElement = raw.root ?? raw;

    if (typeof rootElement === 'object' && rootElement !== null && rootElement.type) {
      // Nested tree format — convert to flat
      const flat = nestedToFlat(rootElement);
      // Preserve state if present in the original spec
      if (raw.state) {
        return { ...flat, state: raw.state } as Spec;
      }
      return flat as Spec;
    }

    if (typeof raw.root === 'string' && raw.elements) {
      // Already flat format
      return raw as Spec;
    }

    return null;
  } catch {
    return null;
  }
}

const JsonRenderBlock = React.memo(function JsonRenderBlock({ spec }: JsonRenderBlockProps) {
  const { parsedSpec, error } = useMemo(() => {
    try {
      const trimmed = spec.trim();

      if (trimmed.length === 0) {
        return { parsedSpec: null, error: null };
      }

      if (trimmed.length > MAX_SPEC_CHARS) {
        return {
          parsedSpec: null,
          error: `Spec too large (${trimmed.length.toLocaleString()} chars). Maximum is ${MAX_SPEC_CHARS.toLocaleString()} chars.`,
        };
      }

      if (isJsonlFormat(trimmed)) {
        const lineCount = trimmed.split('\n').length;
        if (lineCount > MAX_JSONL_LINES) {
          return {
            parsedSpec: null,
            error: `Spec too large (${lineCount.toLocaleString()} lines). Maximum is ${MAX_JSONL_LINES.toLocaleString()} lines.`,
          };
        }

        // JSONL streaming patch format
        const { spec: result, recoveredLineCount } = parseJsonlSpec(trimmed);
        if (result?.root && result.elements) {
          const elementCount = Object.keys(result.elements).length;
          if (elementCount > MAX_ELEMENTS) {
            return {
              parsedSpec: null,
              error: `Spec too large (${elementCount.toLocaleString()} elements). Maximum is ${MAX_ELEMENTS.toLocaleString()} elements.`,
            };
          }

          const unknown = getUnknownComponentTypes(result.elements);
          if (unknown.length > 0) {
            const preview = unknown
              .slice(0, 8)
              .map((u) => `${u.type} (${u.elementId})`)
              .join(', ');
            return {
              parsedSpec: null,
              error: `Unknown component type(s): ${preview}${unknown.length > 8 ? ', …' : ''}`,
            };
          }

          // Surface light debug info without noisy logs
          if (recoveredLineCount > 0 && import.meta.env.DEV && import.meta.env.MODE !== 'test') {
            console.debug(`[json-render] recovered ${recoveredLineCount} malformed JSONL line(s)`);
          }

          return { parsedSpec: result, error: null };
        }
        return { parsedSpec: null, error: 'Invalid JSONL spec: missing root or elements' };
      }

      // Try nested JSON tree format
      const result = parseNestedSpec(trimmed);
      if (result) {
        if (result.elements) {
          const elementCount = Object.keys(result.elements).length;
          if (elementCount > MAX_ELEMENTS) {
            return {
              parsedSpec: null,
              error: `Spec too large (${elementCount.toLocaleString()} elements). Maximum is ${MAX_ELEMENTS.toLocaleString()} elements.`,
            };
          }

          const unknown = getUnknownComponentTypes(result.elements);
          if (unknown.length > 0) {
            const preview = unknown
              .slice(0, 8)
              .map((u) => `${u.type} (${u.elementId})`)
              .join(', ');
            return {
              parsedSpec: null,
              error: `Unknown component type(s): ${preview}${unknown.length > 8 ? ', …' : ''}`,
            };
          }
        }

        return { parsedSpec: result, error: null };
      }

      // Unknown content: don't silently hide it. Show an error so we can debug.
      return { parsedSpec: null, error: 'Invalid spec: unrecognized format' };
    } catch (e) {
      return {
        parsedSpec: null,
        error: `Parse error: ${e instanceof Error ? e.message : String(e)}`,
      };
    }
  }, [spec]);

  if (error) {
    return (
      <div className="rounded-md border border-border-danger bg-background-danger-muted p-4 text-sm text-text-danger">
        <strong>json-render error:</strong> {error}
      </div>
    );
  }

  if (!parsedSpec || !parsedSpec.root) return null;

  type CatalogRendererProps = React.ComponentProps<typeof CatalogRenderer>;

  return (
    <div className="my-4 json-render-block w-full min-w-0">
      <CatalogRenderer
        spec={parsedSpec as CatalogRendererProps['spec']}
        state={(parsedSpec.state ?? {}) as CatalogRendererProps['state']}
      />
    </div>
  );
});

export default JsonRenderBlock;
