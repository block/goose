import { maxBy as maxBy$1 } from '../../array/maxBy.mjs';
import { identity } from '../../function/identity.mjs';
import { iteratee } from '../util/iteratee.mjs';

function maxBy(items, iteratee$1) {
    if (items == null) {
        return undefined;
    }
    return maxBy$1(Array.from(items), iteratee(iteratee$1 ?? identity));
}

export { maxBy };
