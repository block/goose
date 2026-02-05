import { inRange as inRange$1 } from '../../math/inRange.mjs';

function inRange(value, minimum, maximum) {
    if (!minimum) {
        minimum = 0;
    }
    if (maximum != null && !maximum) {
        maximum = 0;
    }
    if (minimum != null && typeof minimum !== 'number') {
        minimum = Number(minimum);
    }
    if (maximum == null && minimum === 0) {
        return false;
    }
    if (maximum != null && typeof maximum !== 'number') {
        maximum = Number(maximum);
    }
    if (maximum != null && minimum > maximum) {
        [minimum, maximum] = [maximum, minimum];
    }
    if (minimum === maximum) {
        return false;
    }
    return inRange$1(value, minimum, maximum);
}

export { inRange };
