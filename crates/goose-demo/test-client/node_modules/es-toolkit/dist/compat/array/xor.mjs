import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { toArray } from '../util/toArray.mjs';

function xor(...arrays) {
    const itemCounts = new Map();
    for (let i = 0; i < arrays.length; i++) {
        const array = arrays[i];
        if (!isArrayLikeObject(array)) {
            continue;
        }
        const itemSet = new Set(toArray(array));
        for (const item of itemSet) {
            if (!itemCounts.has(item)) {
                itemCounts.set(item, 1);
            }
            else {
                itemCounts.set(item, itemCounts.get(item) + 1);
            }
        }
    }
    const result = [];
    for (const [item, count] of itemCounts) {
        if (count === 1) {
            result.push(item);
        }
    }
    return result;
}

export { xor };
