import { reduceAnsiCodes } from "./reduce.js";
/** Returns the combination of ANSI codes needed to undo the given ANSI codes */
export function undoAnsiCodes(codes) {
    return reduceAnsiCodes(codes)
        .reverse()
        .map((code) => ({
        ...code,
        code: code.endCode,
    }));
}
//# sourceMappingURL=undo.js.map