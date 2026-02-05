import { trim as trim$1 } from '../../string/trim.mjs';

function trim(str, chars, guard) {
    if (str == null) {
        return '';
    }
    if (guard != null || chars == null) {
        return str.toString().trim();
    }
    switch (typeof chars) {
        case 'object': {
            if (Array.isArray(chars)) {
                return trim$1(str, chars.flatMap(x => x.toString().split('')));
            }
            else {
                return trim$1(str, chars.toString().split(''));
            }
        }
        default: {
            return trim$1(str, chars.toString().split(''));
        }
    }
}

export { trim };
