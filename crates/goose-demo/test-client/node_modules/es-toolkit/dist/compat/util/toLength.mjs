import { MAX_ARRAY_LENGTH } from '../_internal/MAX_ARRAY_LENGTH.mjs';
import { clamp } from '../math/clamp.mjs';

function toLength(value) {
    if (value == null) {
        return 0;
    }
    const length = Math.floor(Number(value));
    return clamp(length, 0, MAX_ARRAY_LENGTH);
}

export { toLength };
