import isFullwidthCodePoint from "is-fullwidth-code-point";
import { ESCAPES, getEndCode, linkStartCodePrefix, linkStartCodePrefixCharCodes, } from "./ansiCodes.js";
function findNumberIndex(str) {
    for (let index = 0; index < str.length; index++) {
        const charCode = str.charCodeAt(index);
        if (charCode >= 48 && charCode <= 57) {
            return index;
        }
    }
    return -1;
}
function parseLinkCode(string, offset) {
    string = string.slice(offset);
    for (let index = 1; index < linkStartCodePrefixCharCodes.length; index++) {
        if (string.charCodeAt(index) !== linkStartCodePrefixCharCodes[index]) {
            return undefined;
        }
    }
    // This is a link code (with or without the URL part). Find the end of it.
    const endIndex = string.indexOf("\x07", linkStartCodePrefix.length);
    if (endIndex === -1)
        return undefined;
    return string.slice(0, endIndex + 1);
}
function parseAnsiCode(string, offset) {
    string = string.slice(offset, offset + 19);
    const startIndex = findNumberIndex(string);
    if (startIndex !== -1) {
        let endIndex = string.indexOf("m", startIndex);
        if (endIndex === -1) {
            endIndex = string.length;
        }
        return string.slice(0, endIndex + 1);
    }
}
export function tokenize(str, endChar = Number.POSITIVE_INFINITY) {
    const ret = [];
    let index = 0;
    let visible = 0;
    while (index < str.length) {
        const codePoint = str.codePointAt(index);
        if (ESCAPES.has(codePoint)) {
            // TODO: We should probably decide on the next character ("[" or "]") which code path to take.
            const code = parseLinkCode(str, index) || parseAnsiCode(str, index);
            if (code) {
                ret.push({
                    type: "ansi",
                    code,
                    endCode: getEndCode(code),
                });
                index += code.length;
                continue;
            }
        }
        const fullWidth = isFullwidthCodePoint(codePoint);
        const character = String.fromCodePoint(codePoint);
        ret.push({
            type: "char",
            value: character,
            fullWidth,
        });
        index += character.length;
        visible += fullWidth ? 2 : character.length;
        if (visible >= endChar) {
            break;
        }
    }
    return ret;
}
//# sourceMappingURL=tokenize.js.map