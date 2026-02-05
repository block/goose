import { flatten } from '../../array/flatten.mjs';

function concat(...values) {
    return flatten(values);
}

export { concat };
