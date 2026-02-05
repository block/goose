import { keysIn } from './keysIn.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function assignIn(object, ...sources) {
    for (let i = 0; i < sources.length; i++) {
        assignInImpl(object, sources[i]);
    }
    return object;
}
function assignInImpl(object, source) {
    const keys = keysIn(source);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        if (!(key in object) || !isEqualsSameValueZero(object[key], source[key])) {
            object[key] = source[key];
        }
    }
}

export { assignIn };
