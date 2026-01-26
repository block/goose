declare module 'shell-quote' {
  export function parse(cmd: string): unknown[];
  export function quote(args: string[]): string;
}
