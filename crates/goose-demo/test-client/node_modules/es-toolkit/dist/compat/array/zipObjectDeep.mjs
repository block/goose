import { zip } from '../../array/zip.mjs';
import { set } from '../object/set.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function zipObjectDeep(keys, values) {
    const result = {};
    if (!isArrayLike(keys)) {
        return result;
    }
    if (!isArrayLike(values)) {
        values = [];
    }
    const zipped = zip(Array.from(keys), Array.from(values));
    for (let i = 0; i < zipped.length; i++) {
        const [key, value] = zipped[i];
        if (key != null) {
            set(result, key, value);
        }
    }
    return result;
}

export { zipObjectDeep };
