import { toInteger } from './toInteger.mjs';
import { MAX_SAFE_INTEGER } from '../_internal/MAX_SAFE_INTEGER.mjs';
import { clamp } from '../math/clamp.mjs';

function toSafeInteger(value) {
    if (value == null) {
        return 0;
    }
    return clamp(toInteger(value), -MAX_SAFE_INTEGER, MAX_SAFE_INTEGER);
}

export { toSafeInteger };
