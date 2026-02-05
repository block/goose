import { cloneDeepWith as cloneDeepWith$1, copyProperties } from '../../object/cloneDeepWith.mjs';
import { getTag } from '../_internal/getTag.mjs';
import { objectTag, argumentsTag, booleanTag, stringTag, numberTag } from '../_internal/tags.mjs';

function cloneDeepWith(obj, customizer) {
    return cloneDeepWith$1(obj, (value, key, object, stack) => {
        const cloned = customizer?.(value, key, object, stack);
        if (cloned !== undefined) {
            return cloned;
        }
        if (typeof obj !== 'object') {
            return undefined;
        }
        if (getTag(obj) === objectTag && typeof obj.constructor !== 'function') {
            const result = {};
            stack.set(obj, result);
            copyProperties(result, obj, object, stack);
            return result;
        }
        switch (Object.prototype.toString.call(obj)) {
            case numberTag:
            case stringTag:
            case booleanTag: {
                const result = new obj.constructor(obj?.valueOf());
                copyProperties(result, obj);
                return result;
            }
            case argumentsTag: {
                const result = {};
                copyProperties(result, obj);
                result.length = obj.length;
                result[Symbol.iterator] = obj[Symbol.iterator];
                return result;
            }
            default: {
                return undefined;
            }
        }
    });
}

export { cloneDeepWith };
