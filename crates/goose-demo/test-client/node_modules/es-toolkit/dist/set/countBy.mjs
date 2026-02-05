function countBy(set, mapper) {
    const result = new Map();
    for (const value of set) {
        const mappedKey = mapper(value, value, set);
        result.set(mappedKey, (result.get(mappedKey) ?? 0) + 1);
    }
    return result;
}

export { countBy };
