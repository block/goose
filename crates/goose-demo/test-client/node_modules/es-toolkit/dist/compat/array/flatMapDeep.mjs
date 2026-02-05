import { flatMapDepth } from './flatMapDepth.mjs';

function flatMapDeep(collection, iteratee) {
    return flatMapDepth(collection, iteratee, Infinity);
}

export { flatMapDeep };
