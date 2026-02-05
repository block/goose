import { lowerFirst as lowerFirst$1 } from '../../string/lowerFirst.mjs';
import { toString } from '../util/toString.mjs';

function lowerFirst(str) {
    return lowerFirst$1(toString(str));
}

export { lowerFirst };
