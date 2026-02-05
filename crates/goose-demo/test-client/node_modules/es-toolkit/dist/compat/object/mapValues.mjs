import { identity } from '../../function/identity.mjs';
import { mapValues as mapValues$1 } from '../../object/mapValues.mjs';
import { iteratee } from '../util/iteratee.mjs';

function mapValues(object, getNewValue = identity) {
    if (object == null) {
        return {};
    }
    return mapValues$1(object, iteratee(getNewValue));
}

export { mapValues };
