import { invoke } from './invoke.mjs';

function methodOf(object, ...args) {
    return function (path) {
        return invoke(object, path, args);
    };
}

export { methodOf };
