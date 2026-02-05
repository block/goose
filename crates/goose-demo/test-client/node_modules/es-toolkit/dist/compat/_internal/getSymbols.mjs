function getSymbols(object) {
    return Object.getOwnPropertySymbols(object).filter(symbol => Object.prototype.propertyIsEnumerable.call(object, symbol));
}

export { getSymbols };
