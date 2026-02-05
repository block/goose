function isSymbol(value) {
    return typeof value === 'symbol' || value instanceof Symbol;
}

export { isSymbol };
