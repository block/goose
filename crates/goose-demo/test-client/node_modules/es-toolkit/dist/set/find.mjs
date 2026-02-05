function find(set, doesMatch) {
    for (const value of set) {
        if (doesMatch(value, value, set)) {
            return value;
        }
    }
    return undefined;
}

export { find };
