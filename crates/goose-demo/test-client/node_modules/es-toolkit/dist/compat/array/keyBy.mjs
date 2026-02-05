import { reduce } from './reduce.mjs';
import { identity } from '../../function/identity.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isObjectLike } from '../predicate/isObjectLike.mjs';
import { iteratee } from '../util/iteratee.mjs';

function keyBy(collection, iteratee$1) {
    if (!isArrayLike(collection) && !isObjectLike(collection)) {
        return {};
    }
    const keyFn = iteratee(iteratee$1 ?? identity);
    return reduce(collection, (result, value) => {
        const key = keyFn(value);
        result[key] = value;
        return result;
    }, {});
}

export { keyBy };
