function zipObject(keys, values) {
    const result = {};
    for (let i = 0; i < keys.length; i++) {
        result[keys[i]] = values[i];
    }
    return result;
}

export { zipObject };
