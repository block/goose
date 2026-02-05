import { deburr as deburr$1 } from '../../string/deburr.mjs';
import { toString } from '../util/toString.mjs';

function deburr(str) {
    return deburr$1(toString(str));
}

export { deburr };
