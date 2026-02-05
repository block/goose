import { after } from '../../function/after.mjs';
import { isEqualWith as isEqualWith$1 } from '../../predicate/isEqualWith.mjs';

function isEqualWith(a, b, areValuesEqual) {
    if (typeof areValuesEqual !== 'function') {
        areValuesEqual = () => undefined;
    }
    return isEqualWith$1(a, b, (...args) => {
        const result = areValuesEqual(...args);
        if (result !== undefined) {
            return Boolean(result);
        }
        if (a instanceof Map && b instanceof Map) {
            return isEqualWith(Array.from(a), Array.from(b), after(2, areValuesEqual));
        }
        if (a instanceof Set && b instanceof Set) {
            return isEqualWith(Array.from(a), Array.from(b), after(2, areValuesEqual));
        }
    });
}

export { isEqualWith };
