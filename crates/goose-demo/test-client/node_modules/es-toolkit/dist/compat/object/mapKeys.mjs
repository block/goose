import { identity } from '../../function/identity.mjs';
import { mapKeys as mapKeys$1 } from '../../object/mapKeys.mjs';
import { iteratee } from '../util/iteratee.mjs';

function mapKeys(object, getNewKey = identity) {
    if (object == null) {
        return {};
    }
    return mapKeys$1(object, iteratee(getNewKey));
}

export { mapKeys };
