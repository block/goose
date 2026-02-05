import { toNumber } from './toNumber.mjs';

function gt(value, other) {
    if (typeof value === 'string' && typeof other === 'string') {
        return value > other;
    }
    return toNumber(value) > toNumber(other);
}

export { gt };
