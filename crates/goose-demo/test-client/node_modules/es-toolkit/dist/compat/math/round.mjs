import { decimalAdjust } from '../_internal/decimalAdjust.mjs';

function round(number, precision = 0) {
    return decimalAdjust('round', number, precision);
}

export { round };
