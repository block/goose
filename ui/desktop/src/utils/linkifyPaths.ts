import { visit, SKIP } from 'unist-util-visit';
import type { Plugin } from 'unified';
import type { Root, Text, InlineCode, Link, Parent } from 'mdast';

const OPEN_FILE_PROTOCOL = 'open-file://';

const TRAILING_PUNCTUATION_RE = /[.,;:!?'"]+$/;

type PathMatch = [index: number, path: string];
type Separator = '/' | '\\';

function isPathChar(char: string): boolean {
  if (/[a-zA-Z0-9._+@%-]/.test(char)) return true;
  return /\p{L}|\p{N}/u.test(char);
}

function stripTrailingPunctuation(path: string): string {
  return path.replace(TRAILING_PUNCTUATION_RE, '');
}

function stripLineColumnSuffix(path: string): string {
  const match = path.match(/\.[A-Za-z0-9]+(:\d+(?::\d+)?)$/);
  if (match) {
    return path.slice(0, -match[1].length);
  }
  return path;
}

function isUrlPathAt(text: string, index: number): boolean {
  let i = index - 1;
  if (i < 0) return false;

  if (text[i] === '/') {
    i--;
    if (i >= 0 && text[i] === '/') i--;
  }
  if (i < 0 || text[i] !== ':') return false;

  i--;
  const schemeEnd = i;
  while (i >= 0 && /[a-zA-Z0-9+.-]/.test(text[i])) {
    i--;
  }

  const scheme = text.slice(i + 1, schemeEnd + 1);
  return scheme.length > 0 && /[a-zA-Z]/.test(scheme[0]);
}

function isAssignmentEqualsStart(text: string, index: number): boolean {
  if (text[index - 1] !== '=') return false;
  let i = index - 2;
  while (i >= 0 && /[a-zA-Z0-9_.$-]/.test(text[i])) {
    i--;
  }
  return !(i >= 0 && (text[i] === '?' || text[i] === '&'));
}

function isCandidatePathStart(text: string, index: number, afterBlockComment: boolean): boolean {
  if (index === 0) return true;
  if (isUrlPathAt(text, index)) return false;
  const prev = text[index - 1];
  if (prev === '=') return isAssignmentEqualsStart(text, index);
  if (/[\s('"`[(,;]/.test(prev)) return true;
  return afterBlockComment;
}

function couldStartPath(text: string, index: number): boolean {
  const char = text[index];
  return char === '/' || char === '~' || /[A-Za-z]/.test(char);
}

function getLastToken(text: string, segmentStart: number, beforeIndex: number): string {
  const segment = text.slice(segmentStart, beforeIndex);
  const lastSpace = segment.lastIndexOf(' ');
  return lastSpace === -1 ? segment : segment.slice(lastSpace + 1);
}

function isParenthesizedSuffix(token: string): boolean {
  return /^\([^)]+\)$/.test(token);
}

function isPathLikeSpacedWord(word: string, prevToken: string): boolean {
  if (isParenthesizedSuffix(prevToken)) return false;
  if (/^\d+$/.test(word)) {
    return word.length >= 4;
  }
  if (/^[A-Z]/.test(word)) {
    if (/\.[A-Za-z0-9]+$/.test(word)) {
      return /^[A-Z]/.test(prevToken);
    }
    return (
      /^[A-Z]/.test(prevToken) &&
      word.length >= 4 &&
      prevToken.length >= 2 &&
      prevToken.length <= 3
    );
  }
  if (/\.[A-Za-z0-9]+$/.test(word) && /^[a-z]/.test(prevToken)) {
    return true;
  }
  return (
    word.length >= 4 &&
    prevToken.length >= 2 &&
    prevToken.length <= 2 &&
    !prevToken.includes('.')
  );
}

function readExtensionWordAhead(text: string, fromIndex: number): boolean {
  let i = fromIndex;
  while (i < text.length) {
    if (text[i] !== ' ') return false;
    i++;
    let j = i;
    while (j < text.length && isPathChar(text[j])) {
      j++;
    }
    if (j === i) return false;
    const word = text.slice(i, j).replace(TRAILING_PUNCTUATION_RE, '');
    if (/\.[A-Za-z0-9]+$/.test(word) && /^[a-z]/.test(word)) return true;
    if (!/^[a-z][a-z0-9]*$/.test(word)) return false;
    i = j;
  }
  return false;
}

function isSpacedFilenameContinuation(
  word: string,
  prevToken: string,
  text: string,
  endIndex: number
): boolean {
  if (/^\[[^\]]+\]$/.test(word)) {
    const inner = word.slice(1, -1);
    return (
      prevToken.length >= 2 &&
      (/^[a-z][a-z0-9]*$/.test(inner) || /^\d+$/.test(inner))
    );
  }
  if (isPathLikeSpacedWord(word, prevToken)) return true;
  return (
    /^[a-z][a-z0-9]*$/.test(word) &&
    /^[a-z]/.test(prevToken) &&
    readExtensionWordAhead(text, endIndex)
  );
}

function isLinkLikeParent(parent: Parent | undefined): boolean {
  return parent?.type === 'link' || parent?.type === 'linkReference';
}

function readParenthesizedSuffix(text: string, spaceIndex: number): number {
  if (text[spaceIndex] !== ' ') return spaceIndex;

  let i = spaceIndex + 1;
  if (text[i] !== '(') return spaceIndex;

  i++;
  const contentStart = i;
  while (i < text.length && text[i] !== ')') {
    if (text[i] === '(') return spaceIndex;
    i++;
  }
  if (i >= text.length) return spaceIndex;

  const content = text.slice(contentStart, i);
  if (!/^[a-zA-Z0-9]+$/.test(content)) return spaceIndex;

  i++;
  const afterParen = i;
  while (i < text.length && isPathChar(text[i])) {
    i++;
  }
  if (/^\d+$/.test(content)) {
    return i;
  }
  if (i > afterParen) {
    return i;
  }
  if (/^[a-zA-Z0-9]{1,4}$/.test(content) && /\d/.test(content)) {
    return afterParen;
  }
  return spaceIndex;
}

function isConnectiveSpacedWord(word: string): boolean {
  return /^(?:and|or)$/i.test(word);
}

function readSpacedContinuation(
  text: string,
  spaceIndex: number,
  segmentStart: number,
  separator: Separator
): number {
  if (text[spaceIndex] !== ' ') return spaceIndex;

  let j = spaceIndex + 1;
  while (j < text.length && isPathChar(text[j])) {
    j++;
  }
  const bracketEnd = readBracketedSuffix(text, j);
  if (bracketEnd > j) {
    j = bracketEnd;
    while (j < text.length && isPathChar(text[j])) {
      j++;
    }
  }
  while (j > spaceIndex + 1 && /[.,;:!?)'"]/.test(text[j - 1] ?? '')) {
    j--;
  }
  if (j === spaceIndex + 1) return spaceIndex;

  const word = text.slice(spaceIndex + 1, j);
  const prevToken = getLastToken(text, segmentStart, spaceIndex);
  const after = text[j];

  if (after === separator) {
    return isConnectiveSpacedWord(word) ? spaceIndex : j;
  }

  if (after === undefined || /[.,;:!?)'\]"]/.test(after)) {
    return isSpacedFilenameContinuation(word, prevToken, text, j) ? j : spaceIndex;
  }

  if (after !== undefined && /\s/.test(after)) {
    if (after === ' ') {
      const rest = text.slice(j).trimStart();
      if (rest.startsWith(separator)) return spaceIndex;
    }
    return isSpacedFilenameContinuation(word, prevToken, text, j) ? j : spaceIndex;
  }

  return spaceIndex;
}

function isFilenamePunctuation(char: string): boolean {
  return char === ',' || char === "'" || char === ':' || char === '?';
}

function readBracketedSuffix(text: string, bracketIndex: number): number {
  if (text[bracketIndex] !== '[') return bracketIndex;

  let j = bracketIndex + 1;
  if (j >= text.length) return bracketIndex;

  while (j < text.length && isPathChar(text[j])) {
    j++;
  }
  if (j >= text.length || text[j] !== ']') return bracketIndex;

  return j + 1;
}

function readSegment(text: string, start: number, separator: Separator): { end: number } | null {
  let i = start;
  if (i >= text.length || !isPathChar(text[i])) return null;

  while (i < text.length) {
    if (isPathChar(text[i])) {
      i++;
      continue;
    }
    const bracketEnd = readBracketedSuffix(text, i);
    if (bracketEnd > i) {
      i = bracketEnd;
      continue;
    }
    if (
      isFilenamePunctuation(text[i]) &&
      i + 1 < text.length &&
      isPathChar(text[i + 1])
    ) {
      i++;
      continue;
    }
    break;
  }

  while (i < text.length && text[i] === ' ') {
    const parenEnd = readParenthesizedSuffix(text, i);
    if (parenEnd > i) {
      i = parenEnd;
      continue;
    }
    const continuationEnd = readSpacedContinuation(text, i, start, separator);
    if (continuationEnd === i) break;
    i = continuationEnd;
  }

  return i > start ? { end: i } : null;
}

function isPathTerminator(char: string): boolean {
  return /[.,;:!?'"`)\]]/.test(char);
}

function isPathBoundary(char: string): boolean {
  return /\s/.test(char) || isPathTerminator(char);
}

function parsePathAt(text: string, index: number): PathMatch | null {
  let i = index;
  let separator: Separator = '/';
  let minSegments: number;

  if (text[i] === '/') {
    i++;
    minSegments = 1;
  } else if (text[i] === '~' && text[i + 1] === '/') {
    i += 2;
    minSegments = 1;
  } else if (/[A-Za-z]/.test(text[i] ?? '') && text[i + 1] === ':') {
    i += 2;
    if (text[i] !== '/' && text[i] !== '\\') {
      return null;
    }
    separator = text[i] as Separator;
    i++;
    minSegments = 1;
  } else {
    return null;
  }

  let segmentCount = 0;
  while (i < text.length) {
    const segment = readSegment(text, i, separator);
    if (!segment) {
      if (segmentCount >= minSegments && i < text.length) {
        const next = text[i];
        if (next !== separator && !isPathBoundary(next)) {
          return null;
        }
      }
      break;
    }
    i = segment.end;
    segmentCount++;
    if (i < text.length && text[i] === separator) {
      i++;
      continue;
    }
    if (i < text.length && !isPathBoundary(text[i])) {
      return null;
    }
    break;
  }

  if (segmentCount < minSegments) return null;

  const path = stripLineColumnSuffix(stripTrailingPunctuation(text.slice(index, i)));
  if (path.length === 0) return null;

  return [index, path];
}

export function findPaths(text: string): PathMatch[] {
  const matches: PathMatch[] = [];
  let afterBlockComment = false;

  for (let i = 0; i < text.length; i++) {
    if (text[i] === '/' && text[i + 1] === '*') {
      i += 2;
      while (i < text.length - 1 && !(text[i] === '*' && text[i + 1] === '/')) {
        i++;
      }
      i += 1;
      afterBlockComment = true;
      continue;
    }

    if (
      couldStartPath(text, i) &&
      isCandidatePathStart(text, i, afterBlockComment)
    ) {
      const match = parsePathAt(text, i);
      if (match) {
        matches.push(match);
        i = match[0] + match[1].length - 1;
        afterBlockComment = false;
        continue;
      }
    }

    afterBlockComment = false;
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
    visit(tree, (node, index, parent) => {
      if (node.type === 'link' || node.type === 'linkReference') {
        return SKIP;
      }

      if (node.type !== 'text' && node.type !== 'inlineCode') {
        return undefined;
      }
      if (index === undefined || !parent || isLinkLikeParent(parent)) {
        return undefined;
      }
      linkifyNode(node, index, parent);
      return undefined;
    });
  };
};

export { OPEN_FILE_PROTOCOL };

export function decodeFileLinkHref(href: string): string | undefined {
  if (!href.startsWith(OPEN_FILE_PROTOCOL)) return undefined;
  try {
    return decodeURIComponent(href.slice(OPEN_FILE_PROTOCOL.length));
  } catch {
    return undefined;
  }
}

export function isTrustedGeneratedFileLink(href: string, label: string): boolean {
  const filePath = decodeFileLinkHref(href);
  return filePath !== undefined && label === filePath;
}
