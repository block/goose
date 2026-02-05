function isNode() {
    return typeof process !== 'undefined' && process?.versions?.node != null;
}

export { isNode };
