type ReplaceFunction = (match: string, ...args: any[]) => string;
declare function replace(string: string, pattern: RegExp | string, replacement: ReplaceFunction | string): string;
declare function replace(pattern: RegExp | string, replacement: ReplaceFunction | string): string;

export { replace };
