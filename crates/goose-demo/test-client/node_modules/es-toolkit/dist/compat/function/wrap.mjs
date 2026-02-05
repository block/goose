import { identity } from '../../function/identity.mjs';
import { isFunction } from '../../predicate/isFunction.mjs';

function wrap(value, wrapper) {
    return function (...args) {
        const wrapFn = isFunction(wrapper) ? wrapper : identity;
        return wrapFn.apply(this, [value, ...args]);
    };
}

export { wrap };
