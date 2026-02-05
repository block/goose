function at(arr, indices) {
    const result = new Array(indices.length);
    const length = arr.length;
    for (let i = 0; i < indices.length; i++) {
        let index = indices[i];
        index = Number.isInteger(index) ? index : Math.trunc(index) || 0;
        if (index < 0) {
            index += length;
        }
        result[i] = arr[index];
    }
    return result;
}

export { at };
