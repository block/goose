import { toString } from '../util/toString.mjs';

function split(string, separator, limit) {
    return toString(string).split(separator, limit);
}

export { split };
