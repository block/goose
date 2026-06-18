import { visit } from 'unist-util-visit';
import type { Plugin } from 'unified';
import type { Root, Text, InlineCode, Link, Parent } from 'mdast';

const OPEN_FILE_PROTOCOL = 'open-file://';

const TRAILING_PUNCTUATION_RE = /[.,;:!?)'\]"]+$/;

type PathMatch = [index: number, path: string];
type Separator = '/' | '\\';

function isPathChar(char: string): boolean {
  return /[a-zA-Z0-9._+-]/.test(char);
}

function stripTrailingPunctuation(path: string): string {
  return path.replace(TRAILING_PUNCTUATION_RE, '');
}

function isUrlPath(text: string, index: number): boolean {
  const before = text.slice(0, index);
  return /[a-zA-Z][a-zA-Z0-9+.-]*:\/\/?$/.test(before);
}

function isValidPathStart(text: string, index: number): boolean {
  if (index === 0) return true;
  if (isUrlPath(text, index)) return false;
  const prev = text[index - 1];
  if (/[\s('"`[(,;]/.test(prev)) return true;
  const before = text.slice(0, index);
  return /\/\*.*?\*\/$/.test(before);
}

function readSpacedContinuation(text: string, spaceIndex: number, separator: Separator): number {
  if (text[spaceIndex] !== ' ') return spaceIndex;

  let j = spaceIndex + 1;
  while (j < text.length && isPathChar(text[j])) {
    j++;
  }
  if (j === spaceIndex + 1) return spaceIndex;

  const after = text[j];
  if (after === separator) return j;
  if (after === undefined || /[.,;:!?)'\]"]/.test(after)) return j;

  if (after === ' ') {
    const rest = text.slice(j).trimStart();
    if (rest.startsWith(separator)) return spaceIndex;

    const word = text.slice(spaceIndex + 1, j);
    if (/[A-Z0-9._+-]/.test(word)) return j;

    if (/^[a-z][a-z0-9]*$/.test(word) && (rest === '' || /^[.,;:!?)'\]"]/.test(rest))) {
      return j;
    }
    return spaceIndex;
  }

  return spaceIndex;
}

function readSegment(text: string, start: number, separator: Separator): { end: number } | null {
  let i = start;
  if (i >= text.length || !isPathChar(text[i])) return null;

  while (i < text.length && isPathChar(text[i])) {
    i++;
  }

  while (i < text.length && text[i] === ' ') {
    const continuationEnd = readSpacedContinuation(text, i, separator);
    if (continuationEnd === i) break;
    i = continuationEnd;
  }

  return i > start ? { end: i } : null;
}

function tryParsePath(text: string, index: number): PathMatch | null {
  if (!isValidPathStart(text, index)) return null;

  let i = index;
  let separator: Separator = '/';
  let minSegments: number;

  if (text[i] === '/') {
    i++;
    minSegments = 2;
  } else if (text[i] === '~' && text[i + 1] === '/') {
    i += 2;
    minSegments = 1;
  } else if (/[A-Za-z]/.test(text[i] ?? '') && text[i + 1] === ':') {
    i += 2;
    if (text[i] === '/' || text[i] === '\\') {
      separator = text[i] as Separator;
      i++;
    }
    minSegments = 2;
  } else {
    return null;
  }

  let segmentCount = 0;
  while (i < text.length) {
    const segment = readSegment(text, i, separator);
    if (!segment) break;
    i = segment.end;
    segmentCount++;
    if (i < text.length && text[i] === separator) {
      i++;
      continue;
    }
    break;
  }

  if (segmentCount < minSegments) return null;

  const path = stripTrailingPunctuation(text.slice(index, i));
  if (path.length === 0) return null;

  return [index, path];
}

export function findPaths(text: string): PathMatch[] {
  const matches: PathMatch[] = [];
  for (let i = 0; i < text.length; i++) {
    const match = tryParsePath(text, i);
    if (match) {
      matches.push(match);
      i = match[0] + match[1].length - 1;
    }
  }
  return matches;
}

function linkifyNode(node: Text | InlineCode, index: number, parent: Parent): void {
  const text = node.value;
  const paths = findPaths(text);
  if (paths.length === 0) return;

  const newNodes: Array<Text | InlineCode | Link> = [];
  let lastIndex = 0;
  for (const [pathIndex, path] of paths) {
    if (pathIndex > lastIndex) {
      newNodes.push({
        type: node.type,
        value: text.slice(lastIndex, pathIndex),
      } as Text | InlineCode);
    }
    newNodes.push({
      type: 'link',
      url: OPEN_FILE_PROTOCOL + encodeURI(path),
      title: null,
      children: [
        {
          type: 'text',
          value: path,
        },
      ],
    });
    lastIndex = pathIndex + path.length;
  }
  if (lastIndex < text.length) {
    newNodes.push({
      type: node.type,
      value: text.slice(lastIndex),
    } as Text | InlineCode);
  }
  parent.children.splice(index, 1, ...newNodes);
}

export const remarkLinkifyPaths: Plugin<[], Root> = function () {
  return (tree: Root) => {
    visit(tree, 'text', (node: Text, index: number | undefined, parent: Parent | undefined) => {
      if (index === undefined || !parent || parent.type === 'link') return;
      linkifyNode(node, index, parent);
    });
    visit(tree, 'inlineCode', (node: InlineCode, index: number | undefined, parent: Parent | undefined) => {
      if (index === undefined || !parent || parent.type === 'link') return;
      linkifyNode(node, index, parent);
    });
  };
};

export { OPEN_FILE_PROTOCOL };
