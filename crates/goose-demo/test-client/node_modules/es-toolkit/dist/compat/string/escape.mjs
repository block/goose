import { escape as escape$1 } from '../../string/escape.mjs';
import { toString } from '../util/toString.mjs';

function escape(string) {
    return escape$1(toString(string));
}

export { escape };
