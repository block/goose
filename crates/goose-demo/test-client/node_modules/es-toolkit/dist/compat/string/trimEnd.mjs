import { trimEnd as trimEnd$1 } from '../../string/trimEnd.mjs';

function trimEnd(str, chars, guard) {
    if (str == null) {
        return '';
    }
    if (guard != null || chars == null) {
        return str.toString().trimEnd();
    }
    return trimEnd$1(str, chars.toString().split(''));
}

export { trimEnd };
