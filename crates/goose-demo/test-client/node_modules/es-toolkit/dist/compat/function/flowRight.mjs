import { flatten } from '../../array/flatten.mjs';
import { flowRight as flowRight$1 } from '../../function/flowRight.mjs';

function flowRight(...funcs) {
    const flattenFuncs = flatten(funcs, 1);
    if (flattenFuncs.some(func => typeof func !== 'function')) {
        throw new TypeError('Expected a function');
    }
    return flowRight$1(...flattenFuncs);
}

export { flowRight };
