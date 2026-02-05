function some(map, doesMatch) {
    for (const [key, value] of map) {
        if (doesMatch(value, key, map)) {
            return true;
        }
    }
    return false;
}

export { some };
