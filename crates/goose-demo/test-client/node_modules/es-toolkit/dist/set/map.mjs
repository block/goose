function map(set, getNewValue) {
    const result = new Set();
    for (const value of set) {
        const newValue = getNewValue(value, value, set);
        result.add(newValue);
    }
    return result;
}

export { map };
