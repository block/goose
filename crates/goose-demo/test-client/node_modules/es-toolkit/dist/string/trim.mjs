import { trimEnd } from './trimEnd.mjs';
import { trimStart } from './trimStart.mjs';

function trim(str, chars) {
    if (chars === undefined) {
        return str.trim();
    }
    return trimStart(trimEnd(str, chars), chars);
}

export { trim };
