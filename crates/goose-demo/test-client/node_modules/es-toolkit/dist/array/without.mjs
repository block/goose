import { difference } from './difference.mjs';

function without(array, ...values) {
    return difference(array, values);
}

export { without };
