import { shuffle as shuffle$1 } from '../../array/shuffle.mjs';
import { values } from '../object/values.mjs';
import { isArray } from '../predicate/isArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isNil } from '../predicate/isNil.mjs';
import { isObjectLike } from '../predicate/isObjectLike.mjs';

function shuffle(collection) {
    if (isNil(collection)) {
        return [];
    }
    if (isArray(collection)) {
        return shuffle$1(collection);
    }
    if (isArrayLike(collection)) {
        return shuffle$1(Array.from(collection));
    }
    if (isObjectLike(collection)) {
        return shuffle$1(values(collection));
    }
    return [];
}

export { shuffle };
