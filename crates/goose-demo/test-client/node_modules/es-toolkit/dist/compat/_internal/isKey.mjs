import { isSymbol } from '../predicate/isSymbol.mjs';

const regexIsDeepProp = /\.|\[(?:[^[\]]*|(["'])(?:(?!\1)[^\\]|\\.)*?\1)\]/;
const regexIsPlainProp = /^\w*$/;
function isKey(value, object) {
    if (Array.isArray(value)) {
        return false;
    }
    if (typeof value === 'number' || typeof value === 'boolean' || value == null || isSymbol(value)) {
        return true;
    }
    return ((typeof value === 'string' && (regexIsPlainProp.test(value) || !regexIsDeepProp.test(value))) ||
        (object != null && Object.hasOwn(object, value)));
}

export { isKey };
