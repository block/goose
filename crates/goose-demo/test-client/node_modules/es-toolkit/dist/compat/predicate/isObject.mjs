function isObject(value) {
    return value !== null && (typeof value === 'object' || typeof value === 'function');
}

export { isObject };
