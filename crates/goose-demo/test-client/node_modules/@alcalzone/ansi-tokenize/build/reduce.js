import ansiStyles from "ansi-styles";
import { endCodesSet } from "./ansiCodes.js";
/** Reduces the given array of ANSI codes to the minimum necessary to render with the same style */
export function reduceAnsiCodes(codes) {
    return reduceAnsiCodesIncremental([], codes);
}
/** Like {@link reduceAnsiCodes}, but assumes that `codes` is already reduced. Further reductions are only done for the items in `newCodes`. */
export function reduceAnsiCodesIncremental(codes, newCodes) {
    let ret = [...codes];
    for (const code of newCodes) {
        if (code.code === ansiStyles.reset.open) {
            // Reset code, disable all codes
            ret = [];
        }
        else if (endCodesSet.has(code.code)) {
            // This is an end code, disable all matching start codes
            ret = ret.filter((retCode) => retCode.endCode !== code.code);
        }
        else {
            // This is a start code. Disable all styles this "overrides", then enable it
            ret = ret.filter((retCode) => retCode.endCode !== code.endCode);
            ret.push(code);
        }
    }
    return ret;
}
//# sourceMappingURL=reduce.js.map