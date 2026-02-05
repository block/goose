import { remove as remove$1 } from '../../array/remove.mjs';
import { identity } from '../../function/identity.mjs';
import { iteratee } from '../util/iteratee.mjs';

function remove(arr, shouldRemoveElement = identity) {
    return remove$1(arr, iteratee(shouldRemoveElement));
}

export { remove };
