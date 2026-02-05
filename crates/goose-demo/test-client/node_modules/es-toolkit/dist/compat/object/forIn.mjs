import { identity } from '../../function/identity.mjs';

function forIn(object, iteratee = identity) {
    if (object == null) {
        return object;
    }
    for (const key in object) {
        const result = iteratee(object[key], key, object);
        if (result === false) {
            break;
        }
    }
    return object;
}

export { forIn };
