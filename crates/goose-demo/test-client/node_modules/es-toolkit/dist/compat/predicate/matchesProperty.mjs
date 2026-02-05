import { isMatch } from './isMatch.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { cloneDeep } from '../object/cloneDeep.mjs';
import { get } from '../object/get.mjs';
import { has } from '../object/has.mjs';

function matchesProperty(property, source) {
    switch (typeof property) {
        case 'object': {
            if (Object.is(property?.valueOf(), -0)) {
                property = '-0';
            }
            break;
        }
        case 'number': {
            property = toKey(property);
            break;
        }
    }
    source = cloneDeep(source);
    return function (target) {
        const result = get(target, property);
        if (result === undefined) {
            return has(target, property);
        }
        if (source === undefined) {
            return result === undefined;
        }
        return isMatch(result, source);
    };
}

export { matchesProperty };
