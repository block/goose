import { toInteger } from '../util/toInteger.mjs';

function before(n, func) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    let result;
    n = toInteger(n);
    return function (...args) {
        if (--n > 0) {
            result = func.apply(this, args);
        }
        if (n <= 1 && func) {
            func = undefined;
        }
        return result;
    };
}

export { before };
