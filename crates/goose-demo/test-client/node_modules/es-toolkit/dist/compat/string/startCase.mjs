import { deburr } from './deburr.mjs';
import { words } from '../../string/words.mjs';
import { normalizeForCase } from '../_internal/normalizeForCase.mjs';

function startCase(str) {
    const words$1 = words(normalizeForCase(deburr(str)).trim());
    let result = '';
    for (let i = 0; i < words$1.length; i++) {
        const word = words$1[i];
        if (result) {
            result += ' ';
        }
        if (word === word.toUpperCase()) {
            result += word;
        }
        else {
            result += word[0].toUpperCase() + word.slice(1).toLowerCase();
        }
    }
    return result;
}

export { startCase };
