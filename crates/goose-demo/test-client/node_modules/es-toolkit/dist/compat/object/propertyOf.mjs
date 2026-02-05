import { get } from './get.mjs';

function propertyOf(object) {
    return function (path) {
        return get(object, path);
    };
}

export { propertyOf };
