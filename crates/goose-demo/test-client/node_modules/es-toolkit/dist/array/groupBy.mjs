function groupBy(arr, getKeyFromItem) {
    const result = {};
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        const key = getKeyFromItem(item, i, arr);
        if (!Object.hasOwn(result, key)) {
            result[key] = [];
        }
        result[key].push(item);
    }
    return result;
}

export { groupBy };
