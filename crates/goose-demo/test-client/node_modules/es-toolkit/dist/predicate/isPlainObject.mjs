function isPlainObject(value) {
    if (!value || typeof value !== 'object') {
        return false;
    }
    const proto = Object.getPrototypeOf(value);
    const hasObjectPrototype = proto === null ||
        proto === Object.prototype ||
        Object.getPrototypeOf(proto) === null;
    if (!hasObjectPrototype) {
        return false;
    }
    return Object.prototype.toString.call(value) === '[object Object]';
}

export { isPlainObject };
