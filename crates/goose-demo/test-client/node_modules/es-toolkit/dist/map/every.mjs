function every(map, doesMatch) {
    for (const [key, value] of map) {
        if (!doesMatch(value, key, map)) {
            return false;
        }
    }
    return true;
}

export { every };
