import { identity } from '../../function/identity.mjs';
import { meanBy as meanBy$1 } from '../../math/meanBy.mjs';
import { iteratee } from '../util/iteratee.mjs';

function meanBy(items, iteratee$1) {
    if (items == null) {
        return NaN;
    }
    return meanBy$1(Array.from(items), iteratee(iteratee$1 ?? identity));
}

export { meanBy };
