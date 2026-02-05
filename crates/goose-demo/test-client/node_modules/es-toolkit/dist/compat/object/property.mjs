import { get } from './get.mjs';

function property(path) {
    return function (object) {
        return get(object, path);
    };
}

export { property };
