import { toNumber } from '../util/toNumber.mjs';

function clamp(value, bound1, bound2) {
    if (bound2 === undefined) {
        bound2 = bound1;
        bound1 = undefined;
    }
    if (bound2 !== undefined) {
        bound2 = toNumber(bound2);
        value = Math.min(value, Number.isNaN(bound2) ? 0 : bound2);
    }
    if (bound1 !== undefined) {
        bound1 = toNumber(bound1);
        value = Math.max(value, Number.isNaN(bound1) ? 0 : bound1);
    }
    return value;
}

export { clamp };
