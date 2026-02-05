import { isIterateeCall } from '../_internal/isIterateeCall.mjs';
import { toFinite } from '../util/toFinite.mjs';

function rangeRight(start, end, step) {
    if (step && typeof step !== 'number' && isIterateeCall(start, end, step)) {
        end = step = undefined;
    }
    start = toFinite(start);
    if (end === undefined) {
        end = start;
        start = 0;
    }
    else {
        end = toFinite(end);
    }
    step = step === undefined ? (start < end ? 1 : -1) : toFinite(step);
    const length = Math.max(Math.ceil((end - start) / (step || 1)), 0);
    const result = new Array(length);
    for (let index = length - 1; index >= 0; index--) {
        result[index] = start;
        start += step;
    }
    return result;
}

export { rangeRight };
