function isBlob(x) {
    if (typeof Blob === 'undefined') {
        return false;
    }
    return x instanceof Blob;
}

export { isBlob };
