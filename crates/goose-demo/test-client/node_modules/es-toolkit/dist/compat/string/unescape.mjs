import { unescape as unescape$1 } from '../../string/unescape.mjs';
import { toString } from '../util/toString.mjs';

function unescape(str) {
    return unescape$1(toString(str));
}

export { unescape };
