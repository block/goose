import { mergeWith } from './mergeWith.mjs';
import { noop } from '../../function/noop.mjs';

function merge(object, ...sources) {
    return mergeWith(object, ...sources, noop);
}

export { merge };
