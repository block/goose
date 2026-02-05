import { toString } from '../util/toString.mjs';

function padStart(str, length = 0, chars = ' ') {
    return toString(str).padStart(length, chars);
}

export { padStart };
