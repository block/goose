import { zip as zip$1 } from '../../array/zip.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function zip(...arrays) {
    if (!arrays.length) {
        return [];
    }
    return zip$1(...arrays.filter(group => isArrayLikeObject(group)));
}

export { zip };
