import { marked } from "marked";
import { markedTerminal } from "marked-terminal";

marked.use(markedTerminal({ width: 76, reflowText: true, tab: 2 }) as any);

export function renderMarkdown(src: string): string {
  if (!src) return "";
  const rendered = marked.parse(src) as string;
  return rendered.replace(/\n+$/, "");
}
