import { unzip } from './unzip.mjs';
import { isFunction } from '../../predicate/isFunction.mjs';

function zipWith(...combine) {
    let iteratee = combine.pop();
    if (!isFunction(iteratee)) {
        combine.push(iteratee);
        iteratee = undefined;
    }
    if (!combine?.length) {
        return [];
    }
    const result = unzip(combine);
    if (iteratee == null) {
        return result;
    }
    return result.map(group => iteratee(...group));
}

export { zipWith };
