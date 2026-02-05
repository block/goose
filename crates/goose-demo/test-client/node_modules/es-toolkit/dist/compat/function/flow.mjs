import { flatten } from '../../array/flatten.mjs';
import { flow as flow$1 } from '../../function/flow.mjs';

function flow(...funcs) {
    const flattenFuncs = flatten(funcs, 1);
    if (flattenFuncs.some(func => typeof func !== 'function')) {
        throw new TypeError('Expected a function');
    }
    return flow$1(...flattenFuncs);
}

export { flow };
