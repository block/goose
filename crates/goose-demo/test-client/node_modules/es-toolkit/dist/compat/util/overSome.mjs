import { iteratee } from './iteratee.mjs';

function overSome(...predicates) {
    return function (...values) {
        for (let i = 0; i < predicates.length; ++i) {
            const predicate = predicates[i];
            if (!Array.isArray(predicate)) {
                if (iteratee(predicate).apply(this, values)) {
                    return true;
                }
                continue;
            }
            for (let j = 0; j < predicate.length; ++j) {
                if (iteratee(predicate[j]).apply(this, values)) {
                    return true;
                }
            }
        }
        return false;
    };
}

export { overSome };
