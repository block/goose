import { keysIn } from './keysIn.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function assignInWith(object, ...sources) {
    let getValueToAssign = sources[sources.length - 1];
    if (typeof getValueToAssign === 'function') {
        sources.pop();
    }
    else {
        getValueToAssign = undefined;
    }
    for (let i = 0; i < sources.length; i++) {
        assignInWithImpl(object, sources[i], getValueToAssign);
    }
    return object;
}
function assignInWithImpl(object, source, getValueToAssign) {
    const keys = keysIn(source);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const objValue = object[key];
        const srcValue = source[key];
        const newValue = getValueToAssign?.(objValue, srcValue, key, object, source) ?? srcValue;
        if (!(key in object) || !isEqualsSameValueZero(objValue, newValue)) {
            object[key] = newValue;
        }
    }
}

export { assignInWith };
