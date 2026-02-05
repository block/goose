function pickBy(obj, shouldPick) {
    const result = {};
    const keys = Object.keys(obj);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = obj[key];
        if (shouldPick(value, key)) {
            result[key] = value;
        }
    }
    return result;
}

export { pickBy };
