import { toPath } from './toPath.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { last } from '../array/last.mjs';
import { get } from '../object/get.mjs';

function invoke(object, path, ...args) {
    args = args.flat(1);
    if (object == null) {
        return;
    }
    switch (typeof path) {
        case 'string': {
            if (typeof object === 'object' && Object.hasOwn(object, path)) {
                return invokeImpl(object, [path], args);
            }
            return invokeImpl(object, toPath(path), args);
        }
        case 'number':
        case 'symbol': {
            return invokeImpl(object, [path], args);
        }
        default: {
            if (Array.isArray(path)) {
                return invokeImpl(object, path, args);
            }
            else {
                return invokeImpl(object, [path], args);
            }
        }
    }
}
function invokeImpl(object, path, args) {
    const parent = get(object, path.slice(0, -1), object);
    if (parent == null) {
        return undefined;
    }
    let lastKey = last(path);
    const lastValue = lastKey?.valueOf();
    if (typeof lastValue === 'number') {
        lastKey = toKey(lastValue);
    }
    else {
        lastKey = String(lastKey);
    }
    const func = get(parent, lastKey);
    return func?.apply(parent, args);
}

export { invoke };
