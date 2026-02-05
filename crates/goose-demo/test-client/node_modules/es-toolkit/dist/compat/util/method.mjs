import { invoke } from './invoke.mjs';

function method(path, ...args) {
    return function (object) {
        return invoke(object, path, args);
    };
}

export { method };
