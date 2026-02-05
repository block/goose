import { undoAnsiCodes } from "./undo.js";
/**
 * Returns the minimum amount of ANSI codes necessary to get from the compound style `from` to `to`.
 * Both `from` and `to` are expected to be reduced.
 */
export function diffAnsiCodes(from, to) {
    const endCodesInTo = new Set(to.map((code) => code.endCode));
    const startCodesInFrom = new Set(from.map((code) => code.code));
    return [
        // Ignore all styles in `from` that are not overwritten or removed by `to`
        // Disable all styles in `from` that are removed in `to`
        ...undoAnsiCodes(from.filter((code) => !endCodesInTo.has(code.endCode))),
        // Add all styles in `to` that don't exist in `from`
        ...to.filter((code) => !startCodesInFrom.has(code.code)),
    ];
}
//# sourceMappingURL=diff.js.map