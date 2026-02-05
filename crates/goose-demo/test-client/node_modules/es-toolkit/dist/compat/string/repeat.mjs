import { isIterateeCall } from '../_internal/isIterateeCall.mjs';
import { MAX_SAFE_INTEGER } from '../_internal/MAX_SAFE_INTEGER.mjs';
import { toInteger } from '../util/toInteger.mjs';
import { toString } from '../util/toString.mjs';

function repeat(str, n, guard) {
    if (guard ? isIterateeCall(str, n, guard) : n === undefined) {
        n = 1;
    }
    else {
        n = toInteger(n);
    }
    if (n < 1 || n > MAX_SAFE_INTEGER) {
        return '';
    }
    return toString(str).repeat(n);
}

export { repeat };
