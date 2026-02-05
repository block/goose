function invert(obj) {
    const result = {};
    const keys = Object.keys(obj);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = obj[key];
        result[value] = key;
    }
    return result;
}

export { invert };
