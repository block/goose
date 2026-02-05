function isPrototype(value) {
    const constructor = value?.constructor;
    const prototype = typeof constructor === 'function' ? constructor.prototype : Object.prototype;
    return value === prototype;
}

export { isPrototype };
