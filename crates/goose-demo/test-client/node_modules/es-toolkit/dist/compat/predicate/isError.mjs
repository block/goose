import { getTag } from '../_internal/getTag.mjs';

function isError(value) {
    return getTag(value) === '[object Error]';
}

export { isError };
