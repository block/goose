import type { AnsiCode } from "./tokenize.js";
/** Reduces the given array of ANSI codes to the minimum necessary to render with the same style */
export declare function reduceAnsiCodes(codes: AnsiCode[]): AnsiCode[];
/** Like {@link reduceAnsiCodes}, but assumes that `codes` is already reduced. Further reductions are only done for the items in `newCodes`. */
export declare function reduceAnsiCodesIncremental(codes: AnsiCode[], newCodes: AnsiCode[]): AnsiCode[];
