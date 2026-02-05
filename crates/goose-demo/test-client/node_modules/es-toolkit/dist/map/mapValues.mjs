function mapValues(map, getNewValue) {
    const result = new Map();
    for (const [key, value] of map) {
        const newValue = getNewValue(value, key, map);
        result.set(key, newValue);
    }
    return result;
}

export { mapValues };
