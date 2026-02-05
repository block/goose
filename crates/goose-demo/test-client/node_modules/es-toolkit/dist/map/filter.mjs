function filter(map, callback) {
    const result = new Map();
    for (const [key, value] of map) {
        if (callback(value, key, map)) {
            result.set(key, value);
        }
    }
    return result;
}

export { filter };
