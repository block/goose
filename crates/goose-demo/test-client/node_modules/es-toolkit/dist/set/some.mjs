function some(set, doesMatch) {
    for (const value of set) {
        if (doesMatch(value, value, set)) {
            return true;
        }
    }
    return false;
}

export { some };
