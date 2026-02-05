function every(set, doesMatch) {
    for (const value of set) {
        if (!doesMatch(value, value, set)) {
            return false;
        }
    }
    return true;
}

export { every };
