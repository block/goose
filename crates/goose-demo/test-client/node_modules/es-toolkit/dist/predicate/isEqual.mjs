import { isEqualWith } from './isEqualWith.mjs';
import { noop } from '../function/noop.mjs';

function isEqual(a, b) {
    return isEqualWith(a, b, noop);
}

export { isEqual };
