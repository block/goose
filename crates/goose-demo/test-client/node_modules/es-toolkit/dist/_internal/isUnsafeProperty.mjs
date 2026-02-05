function isUnsafeProperty(key) {
    return key === '__proto__';
}

export { isUnsafeProperty };
