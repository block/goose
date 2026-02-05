import { isArrayLike } from '../predicate/isArrayLike.mjs';

function fromPairs(pairs) {
    if (!isArrayLike(pairs)) {
        return {};
    }
    const result = {};
    for (let i = 0; i < pairs.length; i++) {
        const [key, value] = pairs[i];
        result[key] = value;
    }
    return result;
}

export { fromPairs };
