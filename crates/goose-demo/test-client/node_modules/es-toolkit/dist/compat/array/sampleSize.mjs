import { sampleSize as sampleSize$1 } from '../../array/sampleSize.mjs';
import { isIterateeCall } from '../_internal/isIterateeCall.mjs';
import { clamp } from '../math/clamp.mjs';
import { toArray } from '../util/toArray.mjs';
import { toInteger } from '../util/toInteger.mjs';

function sampleSize(collection, size, guard) {
    const arrayCollection = toArray(collection);
    if (guard ? isIterateeCall(collection, size, guard) : size === undefined) {
        size = 1;
    }
    else {
        size = clamp(toInteger(size), 0, arrayCollection.length);
    }
    return sampleSize$1(arrayCollection, size);
}

export { sampleSize };
