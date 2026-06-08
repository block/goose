import { visit } from 'unist-util-visit';
import type { Plugin } from 'unified';
import type { Root, Text, Link, Parent } from 'mdast';

const OPEN_FILE_PROTOCOL = 'open-file://';

const UNIX_PATH_RE = /(?:^|[\s('"`\[(,;]|\/\*.*?\*\/)()(\/(?:[a-zA-Z0-9._+-]+\/){1,}[a-zA-Z0-9._+-]+)/g;
const TILDE_PATH_RE = /(?:^|[\s('"`\[(,;]|\/\*.*?\*\/)()(~\/(?:[a-zA-Z0-9._+-]+\/)*[a-zA-Z0-9._+-]+)/g;
const WIN_PATH_RE = /(?:^|[\s('"`\[(,;]|\/\*.*?\*\/)()([A-Za-z]:[\\/](?:[a-zA-Z0-9._+-]+[\\/])+[a-zA-Z0-9._+-]+)/g;

type PathMatch = [index: number, path: string];

function findPaths(text: string): PathMatch[] {
  const matches: PathMatch[] = [];
  for (const re of [UNIX_PATH_RE, TILDE_PATH_RE, WIN_PATH_RE]) {
    let m: RegExpExecArray | null;
    const localRe = new RegExp(re.source, 'g');
    while ((m = localRe.exec(text)) !== null) {
      const prefixLen = m[1].length;
      const path = m[2];
      const index = m.index + prefixLen;
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

export const remarkLinkifyPaths: Plugin<[], Root> = function () {
  return (tree: Root) => {
    visit(tree, 'text', (node: Text, index: number | undefined, parent: Parent | undefined) => {
      if (index === undefined || !parent) return;

      const paths = findPaths(node.value);
      if (paths.length === 0) return;

      const newNodes: Array<Text | Link> = [];
      let lastIndex = 0;
      for (const [pathIndex, path] of paths) {
        if (pathIndex > lastIndex) {
          newNodes.push({
            type: 'text',
            value: node.value.slice(lastIndex, pathIndex),
          });
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
      if (lastIndex < node.value.length) {
        newNodes.push({
          type: 'text',
          value: node.value.slice(lastIndex),
        });
      }
      parent.children.splice(index, 1, ...newNodes);
    });
  };
};

export { OPEN_FILE_PROTOCOL };