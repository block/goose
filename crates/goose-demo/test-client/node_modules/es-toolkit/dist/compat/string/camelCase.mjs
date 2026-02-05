import { deburr } from './deburr.mjs';
import { camelCase as camelCase$1 } from '../../string/camelCase.mjs';
import { normalizeForCase } from '../_internal/normalizeForCase.mjs';

function camelCase(str) {
    return camelCase$1(normalizeForCase(deburr(str)));
}

export { camelCase };
