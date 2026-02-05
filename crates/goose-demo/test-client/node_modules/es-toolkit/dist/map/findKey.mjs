function findKey(map, doesMatch) {
    let result = undefined;
    for (const [key, value] of map) {
        if (doesMatch(value, key, map)) {
            result = key;
            break;
        }
    }
    return result;
}

export { findKey };
