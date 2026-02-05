import { upperFirst as upperFirst$1 } from '../../string/upperFirst.mjs';
import { toString } from '../util/toString.mjs';

function upperFirst(str) {
    return upperFirst$1(toString(str));
}

export { upperFirst };
