function isBuffer(x) {
    return typeof Buffer !== 'undefined' && Buffer.isBuffer(x);
}

export { isBuffer };
