import { get } from './get.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isString } from '../predicate/isString.mjs';

function at(object, ...paths) {
    if (paths.length === 0) {
        return [];
    }
    const allPaths = [];
    for (let i = 0; i < paths.length; i++) {
        const path = paths[i];
        if (!isArrayLike(path) || isString(path)) {
            allPaths.push(path);
            continue;
        }
        for (let j = 0; j < path.length; j++) {
            allPaths.push(path[j]);
        }
    }
    const result = [];
    for (let i = 0; i < allPaths.length; i++) {
        result.push(get(object, allPaths[i]));
    }
    return result;
}

export { at };
