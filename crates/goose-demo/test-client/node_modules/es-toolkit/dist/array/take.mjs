import { toInteger } from '../compat/util/toInteger.mjs';

function take(arr, count, guard) {
    count = guard || count === undefined ? 1 : toInteger(count);
    return arr.slice(0, count);
}

export { take };
