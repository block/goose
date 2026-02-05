import { decimalAdjust } from '../_internal/decimalAdjust.mjs';

function ceil(number, precision = 0) {
    return decimalAdjust('ceil', number, precision);
}

export { ceil };
