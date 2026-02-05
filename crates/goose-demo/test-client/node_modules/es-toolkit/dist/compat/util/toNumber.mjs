import { isSymbol } from '../predicate/isSymbol.mjs';

function toNumber(value) {
    if (isSymbol(value)) {
        return NaN;
    }
    return Number(value);
}

export { toNumber };
