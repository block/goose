import { createSpecStreamCompiler, nestedToFlat } from '@json-render/core';
import React, { useMemo } from 'react';
import { CatalogRenderer } from './setup';

interface JsonRenderBlockProps {
  spec: string;
}

interface Spec {
  root?: string;
  elements?: Record<string, unknown>;
  state?: Record<string, unknown>;
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
        console.warn('[json-render] Recovered malformed JSONL line by stripping trailing brace');
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
function parseJsonlSpec(text: string): Spec {
  const recovered = text.split('\n').map(recoverJsonLine).join('\n');
  const compiler = createSpecStreamCompiler<Spec>();
  compiler.push(`${recovered}\n`);
  return compiler.getResult();
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

      if (isJsonlFormat(trimmed)) {
        // JSONL streaming patch format
        const result = parseJsonlSpec(trimmed);
        if (result?.root && result.elements) {
          return { parsedSpec: result, error: null };
        }
        return { parsedSpec: null, error: 'Invalid JSONL spec: missing root or elements' };
      }

      // Try nested JSON tree format
      const result = parseNestedSpec(trimmed);
      if (result) {
        return { parsedSpec: result, error: null };
      }

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

  return (
    <div className="my-4 json-render-block">
      {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
      <CatalogRenderer spec={parsedSpec as any} />
    </div>
  );
});

export default JsonRenderBlock;
