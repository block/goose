function conformsTo(target, source) {
    if (source == null) {
        return true;
    }
    if (target == null) {
        return Object.keys(source).length === 0;
    }
    const keys = Object.keys(source);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const predicate = source[key];
        const value = target[key];
        if (value === undefined && !(key in target)) {
            return false;
        }
        if (typeof predicate === 'function' && !predicate(value)) {
            return false;
        }
    }
    return true;
}

export { conformsTo };
