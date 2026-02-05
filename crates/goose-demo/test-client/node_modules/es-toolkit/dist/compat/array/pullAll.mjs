import { pull } from '../../array/pull.mjs';

function pullAll(arr, valuesToRemove = []) {
    return pull(arr, Array.from(valuesToRemove));
}

export { pullAll };
