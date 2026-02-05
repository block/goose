import { getTag } from '../_internal/getTag.mjs';

function isArguments(value) {
    return value !== null && typeof value === 'object' && getTag(value) === '[object Arguments]';
}

export { isArguments };
