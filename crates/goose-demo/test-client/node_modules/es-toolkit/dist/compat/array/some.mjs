import { identity } from '../../function/identity.mjs';
import { property } from '../object/property.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

function some(source, predicate, guard) {
    if (!source) {
        return false;
    }
    if (guard != null) {
        predicate = undefined;
    }
    if (predicate == null) {
        predicate = identity;
    }
    const values = Array.isArray(source) ? source : Object.values(source);
    switch (typeof predicate) {
        case 'function': {
            if (!Array.isArray(source)) {
                const keys = Object.keys(source);
                for (let i = 0; i < keys.length; i++) {
                    const key = keys[i];
                    const value = source[key];
                    if (predicate(value, key, source)) {
                        return true;
                    }
                }
                return false;
            }
            for (let i = 0; i < source.length; i++) {
                if (predicate(source[i], i, source)) {
                    return true;
                }
            }
            return false;
        }
        case 'object': {
            if (Array.isArray(predicate) && predicate.length === 2) {
                const key = predicate[0];
                const value = predicate[1];
                const matchFunc = matchesProperty(key, value);
                if (Array.isArray(source)) {
                    for (let i = 0; i < source.length; i++) {
                        if (matchFunc(source[i])) {
                            return true;
                        }
                    }
                    return false;
                }
                return values.some(matchFunc);
            }
            else {
                const matchFunc = matches(predicate);
                if (Array.isArray(source)) {
                    for (let i = 0; i < source.length; i++) {
                        if (matchFunc(source[i])) {
                            return true;
                        }
                    }
                    return false;
                }
                return values.some(matchFunc);
            }
        }
        case 'number':
        case 'symbol':
        case 'string': {
            const propFunc = property(predicate);
            if (Array.isArray(source)) {
                for (let i = 0; i < source.length; i++) {
                    if (propFunc(source[i])) {
                        return true;
                    }
                }
                return false;
            }
            return values.some(propFunc);
        }
    }
}

export { some };
