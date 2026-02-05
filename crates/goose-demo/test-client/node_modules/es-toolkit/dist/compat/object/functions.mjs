import { keys } from './keys.mjs';

function functions(object) {
    if (object == null) {
        return [];
    }
    return keys(object).filter(key => typeof object[key] === 'function');
}

export { functions };
