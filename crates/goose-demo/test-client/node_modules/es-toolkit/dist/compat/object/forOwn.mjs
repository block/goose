import { keys } from './keys.mjs';
import { identity } from '../../function/identity.mjs';

function forOwn(object, iteratee = identity) {
    if (object == null) {
        return object;
    }
    const iterable = Object(object);
    const keys$1 = keys(object);
    for (let i = 0; i < keys$1.length; ++i) {
        const key = keys$1[i];
        if (iteratee(iterable[key], key, iterable) === false) {
            break;
        }
    }
    return object;
}

export { forOwn };
