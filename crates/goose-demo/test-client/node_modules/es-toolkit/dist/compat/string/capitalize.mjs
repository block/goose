import { capitalize as capitalize$1 } from '../../string/capitalize.mjs';
import { toString } from '../util/toString.mjs';

function capitalize(str) {
    return capitalize$1(toString(str));
}

export { capitalize };
