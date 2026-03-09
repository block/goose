// Detects raw RFC6902 JSONL json-render output (one JSON object per line)
// and wraps it in a fenced ```json-render code block so it renders via
// MarkdownContent -> JsonRenderBlock.
//
// Why: some sessions (and some provider responses) return bare JSONL patch
// lines without the fenced block, which leaves the user with plain text
// instead of rendered UI.

const JSON_RENDER_FENCE_START = '```json-render\n';
const JSON_RENDER_FENCE_END = '\n```';

export function looksLikeJsonlPatch(text: string): boolean {
  const trimmed = text.trimStart();
  if (trimmed.startsWith('```')) return false;
  if (!trimmed.startsWith('{"op"')) return false;

  // Heuristic: first few lines must be JSON objects and must contain /root or /elements
  const lines = trimmed.split('\n').slice(0, 5).filter(Boolean);
  if (lines.length === 0) return false;

  let parsed = 0;
  let hasRootOrElements = false;

  for (const line of lines) {
    const l = line.trim();
    if (!l.startsWith('{') || !l.endsWith('}')) return false;
    try {
      const obj = JSON.parse(l) as { op?: string; path?: string };
      if (obj.op && obj.path) {
        parsed += 1;
        if (
          obj.path === '/root' ||
          obj.path.startsWith('/elements/') ||
          obj.path.startsWith('/state')
        ) {
          hasRootOrElements = true;
        }
      }
    } catch {
      return false;
    }
  }

  return parsed >= 2 && hasRootOrElements;
}

function looksLikeNestedOrFlatSpec(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed.startsWith('{') || !trimmed.endsWith('}')) return false;

  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (!parsed || typeof parsed !== 'object') return false;

    // Flat spec: { root: string, elements: {...} }
    const asAny = parsed as { root?: unknown; elements?: unknown };
    if (typeof asAny.root === 'string' && asAny.elements && typeof asAny.elements === 'object') {
      return true;
    }

    // Nested root element: { type: string, props, children }
    const maybeElement = parsed as { type?: unknown };
    if (typeof maybeElement.type === 'string') return true;
  } catch {
    return false;
  }

  return false;
}

export function looksLikeJsonRenderSpec(text: string): boolean {
  return looksLikeJsonlPatch(text) || looksLikeNestedOrFlatSpec(text);
}

export function wrapBareJsonRender(text: string): string {
  if (!looksLikeJsonlPatch(text)) return text;
  return `${JSON_RENDER_FENCE_START}${text.trim()}${JSON_RENDER_FENCE_END}`;
}
