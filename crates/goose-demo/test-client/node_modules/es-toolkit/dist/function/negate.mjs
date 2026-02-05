function negate(func) {
    return ((...args) => !func(...args));
}

export { negate };
