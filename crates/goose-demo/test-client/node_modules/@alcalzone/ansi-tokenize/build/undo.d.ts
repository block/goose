import type { AnsiCode } from "./tokenize.js";
/** Returns the combination of ANSI codes needed to undo the given ANSI codes */
export declare function undoAnsiCodes(codes: AnsiCode[]): AnsiCode[];
