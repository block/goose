import { filter } from './filter.mjs';
import { identity } from '../../function/identity.mjs';
import { negate } from '../function/negate.mjs';
import { iteratee } from '../util/iteratee.mjs';

function reject(source, predicate = identity) {
    return filter(source, negate(iteratee(predicate)));
}

export { reject };
