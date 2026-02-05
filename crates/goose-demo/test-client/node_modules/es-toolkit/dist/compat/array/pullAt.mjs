import { flattenDepth } from './flattenDepth.mjs';
import { isIndex } from '../_internal/isIndex.mjs';
import { isKey } from '../_internal/isKey.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { at } from '../object/at.mjs';
import { unset } from '../object/unset.mjs';
import { isArray } from '../predicate/isArray.mjs';
import { toPath } from '../util/toPath.mjs';

function pullAt(array, ..._indices) {
    const indices = flattenDepth(_indices, 1);
    if (!array) {
        return Array(indices.length);
    }
    const result = at(array, indices);
    const indicesToPull = indices
        .map(index => (isIndex(index, array.length) ? Number(index) : index))
        .sort((a, b) => b - a);
    for (const index of new Set(indicesToPull)) {
        if (isIndex(index, array.length)) {
            Array.prototype.splice.call(array, index, 1);
            continue;
        }
        if (isKey(index, array)) {
            delete array[toKey(index)];
            continue;
        }
        const path = isArray(index) ? index : toPath(index);
        unset(array, path);
    }
    return result;
}

export { pullAt };
