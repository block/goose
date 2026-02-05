function findValue(map, doesMatch) {
    let result = undefined;
    for (const [key, value] of map) {
        if (doesMatch(value, key, map)) {
            result = value;
            break;
        }
    }
    return result;
}

export { findValue };
