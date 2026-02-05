import type { AnsiCode } from "./tokenize.js";
/**
 * Returns the minimum amount of ANSI codes necessary to get from the compound style `from` to `to`.
 * Both `from` and `to` are expected to be reduced.
 */
export declare function diffAnsiCodes(from: AnsiCode[], to: AnsiCode[]): AnsiCode[];
