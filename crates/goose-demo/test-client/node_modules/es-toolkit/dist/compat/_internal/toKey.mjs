function toKey(value) {
    if (typeof value === 'string' || typeof value === 'symbol') {
        return value;
    }
    if (Object.is(value?.valueOf?.(), -0)) {
        return '-0';
    }
    return String(value);
}

export { toKey };
