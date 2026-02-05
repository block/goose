import { cloneDeep } from './cloneDeep.mjs';
import { defaults } from './defaults.mjs';

function toDefaulted(object, ...sources) {
    const cloned = cloneDeep(object);
    return defaults(cloned, ...sources);
}

export { toDefaulted };
