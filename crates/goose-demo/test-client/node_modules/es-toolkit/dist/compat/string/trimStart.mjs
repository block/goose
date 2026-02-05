import { trimStart as trimStart$1 } from '../../string/trimStart.mjs';

function trimStart(str, chars, guard) {
    if (str == null) {
        return '';
    }
    if (guard != null || chars == null) {
        return str.toString().trimStart();
    }
    return trimStart$1(str, chars.toString().split(''));
}

export { trimStart };
