import { clone } from './clone.mjs';

function cloneWith(value, customizer) {
    if (!customizer) {
        return clone(value);
    }
    const result = customizer(value);
    if (result !== undefined) {
        return result;
    }
    return clone(value);
}

export { cloneWith };
