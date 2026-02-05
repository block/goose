function isPrimitive(value) {
    return value == null || (typeof value !== 'object' && typeof value !== 'function');
}

export { isPrimitive };
