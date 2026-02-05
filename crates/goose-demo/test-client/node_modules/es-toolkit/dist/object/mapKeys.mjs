function mapKeys(object, getNewKey) {
    const result = {};
    const keys = Object.keys(object);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        result[getNewKey(value, key, object)] = value;
    }
    return result;
}

export { mapKeys };
