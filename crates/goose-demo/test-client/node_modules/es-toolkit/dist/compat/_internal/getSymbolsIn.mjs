import { getSymbols } from './getSymbols.mjs';

function getSymbolsIn(object) {
    const result = [];
    while (object) {
        result.push(...getSymbols(object));
        object = Object.getPrototypeOf(object);
    }
    return result;
}

export { getSymbolsIn };
