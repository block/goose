import { identity } from '../../function/identity.mjs';
import { property } from '../object/property.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

function iteratee(value) {
    if (value == null) {
        return identity;
    }
    switch (typeof value) {
        case 'function': {
            return value;
        }
        case 'object': {
            if (Array.isArray(value) && value.length === 2) {
                return matchesProperty(value[0], value[1]);
            }
            return matches(value);
        }
        case 'string':
        case 'symbol':
        case 'number': {
            return property(value);
        }
    }
}

export { iteratee };
