import { isEqualsSameValueZero } from '../_internal/isEqualsSameValueZero.mjs';

function hasValue(map, searchElement) {
    for (const value of map.values()) {
        if (isEqualsSameValueZero(value, searchElement)) {
            return true;
        }
    }
    return false;
}

export { hasValue };
