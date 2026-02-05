function findKey(obj, predicate) {
    const keys = Object.keys(obj);
    return keys.find(key => predicate(obj[key], key, obj));
}

export { findKey };
