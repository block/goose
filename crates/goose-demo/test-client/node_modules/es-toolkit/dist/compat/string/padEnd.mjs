import { toString } from '../util/toString.mjs';

function padEnd(str, length = 0, chars = ' ') {
    return toString(str).padEnd(length, chars);
}

export { padEnd };
