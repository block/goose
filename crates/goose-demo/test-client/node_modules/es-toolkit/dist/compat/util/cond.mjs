import { iteratee } from './iteratee.mjs';
import { isFunction } from '../../predicate/isFunction.mjs';

function cond(pairs) {
    const length = pairs.length;
    const processedPairs = pairs.map(pair => {
        const predicate = pair[0];
        const func = pair[1];
        if (!isFunction(func)) {
            throw new TypeError('Expected a function');
        }
        return [iteratee(predicate), func];
    });
    return function (...args) {
        for (let i = 0; i < length; i++) {
            const pair = processedPairs[i];
            const predicate = pair[0];
            const func = pair[1];
            if (predicate.apply(this, args)) {
                return func.apply(this, args);
            }
        }
    };
}

export { cond };
