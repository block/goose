import { deburr } from './deburr.mjs';
import { upperCase as upperCase$1 } from '../../string/upperCase.mjs';
import { normalizeForCase } from '../_internal/normalizeForCase.mjs';

function upperCase(str) {
    return upperCase$1(normalizeForCase(deburr(str)));
}

export { upperCase };
