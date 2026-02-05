import { toNumber } from '../util/toNumber.mjs';
import { toString } from '../util/toString.mjs';

function divide(value, other) {
    if (value === undefined && other === undefined) {
        return 1;
    }
    if (value === undefined || other === undefined) {
        return value ?? other;
    }
    if (typeof value === 'string' || typeof other === 'string') {
        value = toString(value);
        other = toString(other);
    }
    else {
        value = toNumber(value);
        other = toNumber(other);
    }
    return value / other;
}

export { divide };
