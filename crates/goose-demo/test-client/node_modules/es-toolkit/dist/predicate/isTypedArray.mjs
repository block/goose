function isTypedArray(x) {
    return ArrayBuffer.isView(x) && !(x instanceof DataView);
}

export { isTypedArray };
