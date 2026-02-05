import { isNil } from '../../predicate/isNil.mjs';

function size(target) {
    if (isNil(target)) {
        return 0;
    }
    if (target instanceof Map || target instanceof Set) {
        return target.size;
    }
    return Object.keys(target).length;
}

export { size };
