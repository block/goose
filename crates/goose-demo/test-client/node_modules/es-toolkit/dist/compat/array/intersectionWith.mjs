import { last } from './last.mjs';
import { intersectionWith as intersectionWith$1 } from '../../array/intersectionWith.mjs';
import { uniq } from './uniq.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function intersectionWith(firstArr, ...otherArrs) {
    if (firstArr == null) {
        return [];
    }
    const _comparator = last(otherArrs);
    let comparator = isEqualsSameValueZero;
    let uniq$1 = uniq;
    if (typeof _comparator === 'function') {
        comparator = _comparator;
        uniq$1 = uniqPreserve0;
        otherArrs.pop();
    }
    let result = uniq$1(Array.from(firstArr));
    for (let i = 0; i < otherArrs.length; ++i) {
        const otherArr = otherArrs[i];
        if (otherArr == null) {
            return [];
        }
        result = intersectionWith$1(result, Array.from(otherArr), comparator);
    }
    return result;
}
function uniqPreserve0(arr) {
    const result = [];
    const added = new Set();
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        if (added.has(item)) {
            continue;
        }
        result.push(item);
        added.add(item);
    }
    return result;
}

export { intersectionWith };
