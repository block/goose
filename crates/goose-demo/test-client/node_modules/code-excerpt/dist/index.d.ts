interface Options {
    around?: number;
}
export interface CodeExcerpt {
    line: number;
    value: string;
}
declare const codeExcerpt: (source: string, line: number, options?: Options) => CodeExcerpt[] | undefined;
export default codeExcerpt;
