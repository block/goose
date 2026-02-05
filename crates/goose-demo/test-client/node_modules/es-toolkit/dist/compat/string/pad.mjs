import { pad as pad$1 } from '../../string/pad.mjs';
import { toString } from '../util/toString.mjs';

function pad(str, length, chars) {
    return pad$1(toString(str), length, chars);
}

export { pad };
