import { cloneDeepWithImpl } from './cloneDeepWith.mjs';

function cloneDeep(obj) {
    return cloneDeepWithImpl(obj, undefined, obj, new Map(), undefined);
}

export { cloneDeep };
