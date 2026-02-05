function defaultTo(value, defaultValue) {
    if (value == null || Number.isNaN(value)) {
        return defaultValue;
    }
    return value;
}

export { defaultTo };
