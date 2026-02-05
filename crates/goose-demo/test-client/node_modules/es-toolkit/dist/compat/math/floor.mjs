import { decimalAdjust } from '../_internal/decimalAdjust.mjs';

function floor(number, precision = 0) {
    return decimalAdjust('floor', number, precision);
}

export { floor };
