function toArray(value) {
    return Array.isArray(value) ? value : Array.from(value);
}

export { toArray };
