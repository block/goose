import { toString } from '../util/toString.mjs';

function replace(target, pattern, replacement) {
    if (arguments.length < 3) {
        return toString(target);
    }
    return toString(target).replace(pattern, replacement);
}

export { replace };
