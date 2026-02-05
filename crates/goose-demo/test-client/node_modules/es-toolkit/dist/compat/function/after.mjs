import { toInteger } from '../util/toInteger.mjs';

function after(n, func) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    n = toInteger(n);
    return function (...args) {
        if (--n < 1) {
            return func.apply(this, args);
        }
    };
}

export { after };
