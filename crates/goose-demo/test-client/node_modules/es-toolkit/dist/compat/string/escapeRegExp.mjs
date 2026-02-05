import { escapeRegExp as escapeRegExp$1 } from '../../string/escapeRegExp.mjs';
import { toString } from '../util/toString.mjs';

function escapeRegExp(str) {
    return escapeRegExp$1(toString(str));
}

export { escapeRegExp };
