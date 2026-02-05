function uniqBy(arr, mapper) {
    const map = new Map();
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        const key = mapper(item, i, arr);
        if (!map.has(key)) {
            map.set(key, item);
        }
    }
    return Array.from(map.values());
}

export { uniqBy };
