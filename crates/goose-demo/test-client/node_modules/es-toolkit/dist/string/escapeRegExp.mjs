function escapeRegExp(str) {
    return str.replace(/[\\^$.*+?()[\]{}|]/g, '\\$&');
}

export { escapeRegExp };
