import { identity } from '../../function/identity.mjs';

function forInRight(object, iteratee = identity) {
    if (object == null) {
        return object;
    }
    const keys = [];
    for (const key in object) {
        keys.push(key);
    }
    for (let i = keys.length - 1; i >= 0; i--) {
        const key = keys[i];
        const result = iteratee(object[key], key, object);
        if (result === false) {
            break;
        }
    }
    return object;
}

export { forInRight };
