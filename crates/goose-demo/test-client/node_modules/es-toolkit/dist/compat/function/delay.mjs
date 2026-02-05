import { toNumber } from '../util/toNumber.mjs';

function delay(func, wait, ...args) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    return setTimeout(func, toNumber(wait) || 0, ...args);
}

export { delay };
