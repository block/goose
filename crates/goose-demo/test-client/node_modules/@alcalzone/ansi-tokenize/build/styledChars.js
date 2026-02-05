import { ansiCodesToString } from "./ansiCodes.js";
import { diffAnsiCodes } from "./diff.js";
import { reduceAnsiCodesIncremental } from "./reduce.js";
export function styledCharsFromTokens(tokens) {
    let codes = [];
    const ret = [];
    for (const token of tokens) {
        if (token.type === "ansi") {
            codes = reduceAnsiCodesIncremental(codes, [token]);
        }
        else if (token.type === "char") {
            ret.push({
                ...token,
                styles: [...codes],
            });
        }
    }
    return ret;
}
export function styledCharsToString(chars) {
    let ret = "";
    for (let i = 0; i < chars.length; i++) {
        const char = chars[i];
        if (i === 0) {
            ret += ansiCodesToString(char.styles);
        }
        else {
            ret += ansiCodesToString(diffAnsiCodes(chars[i - 1].styles, char.styles));
        }
        ret += char.value;
        // reset active styles at the end of the string
        if (i === chars.length - 1) {
            ret += ansiCodesToString(diffAnsiCodes(char.styles, []));
        }
    }
    return ret;
}
//# sourceMappingURL=styledChars.js.map