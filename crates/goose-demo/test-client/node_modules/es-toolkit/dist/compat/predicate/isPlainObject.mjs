function isPlainObject(object) {
    if (typeof object !== 'object') {
        return false;
    }
    if (object == null) {
        return false;
    }
    if (Object.getPrototypeOf(object) === null) {
        return true;
    }
    if (Object.prototype.toString.call(object) !== '[object Object]') {
        const tag = object[Symbol.toStringTag];
        if (tag == null) {
            return false;
        }
        const isTagReadonly = !Object.getOwnPropertyDescriptor(object, Symbol.toStringTag)?.writable;
        if (isTagReadonly) {
            return false;
        }
        return object.toString() === `[object ${tag}]`;
    }
    let proto = object;
    while (Object.getPrototypeOf(proto) !== null) {
        proto = Object.getPrototypeOf(proto);
    }
    return Object.getPrototypeOf(object) === proto;
}

export { isPlainObject };
