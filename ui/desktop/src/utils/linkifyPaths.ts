import { visit } from 'unist-util-visit';
import type { Plugin } from 'unified';
import type { Root, Text, InlineCode, Link, Parent } from 'mdast';

const OPEN_FILE_PROTOCOL = 'open-file://';

const UNIX_PATH_RE = /(?:^|[\s('"`[(,;]|\/\*.*?\*\/)?(\/(?:[a-zA-Z0-9._+-]+\/){1,}[a-zA-Z0-9._+-]+)/g;
const TILDE_PATH_RE = /(?:^|[\s('"`[(,;]|\/\*.*?\*\/)?(~\/(?:[a-zA-Z0-9._+-]+\/)*[a-zA-Z0-9._+-]+)/g;
const WIN_PATH_RE = /(?:^|[\s('"`[(,;]|\/\*.*?\*\/)?([A-Za-z]:[\\/](?:[a-zA-Z0-9._+-]+[\\/])+[a-zA-Z0-9._+-]+)/g;

const TRAILING_PUNCTUATION_RE = /[.,;:!?)'\]"]+$/;

type PathMatch = [index: number, path: string];

function stripTrailingPunctuation(path: string): string {
  return path.replace(TRAILING_PUNCTUATION_RE, '');
}

function isUrlPath(text: string, index: number): boolean {
  const before = text.slice(0, index);
  return /[a-zA-Z][a-zA-Z0-9+.-]*:\/\/?$/.test(before);
}

export function findPaths(text: string): PathMatch[] {
  const matches: PathMatch[] = [];
  for (const re of [UNIX_PATH_RE, TILDE_PATH_RE, WIN_PATH_RE]) {
    let m: RegExpExecArray | null;
    const localRe = new RegExp(re.source, 'g');
    while ((m = localRe.exec(text)) !== null) {
      const path = stripTrailingPunctuation(m[1]);
      if (path.length === 0) continue;
      const prefixLen = m[0].length - m[1].length;
      const index = m.index + prefixLen;
      if (isUrlPath(text, index)) continue;
      matches.push([index, path]);
    }
  }
  matches.sort((a, b) => a[0] - b[0]);
  const result: PathMatch[] = [];
  let lastEnd = 0;
  for (const [index, path] of matches) {
    if (index < lastEnd) continue;
    result.push([index, path]);
    lastEnd = index + path.length;
  }
  return result;
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
      url: OPEN_FILE_PROTOCOL + path,
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
