function setToEntries(set) {
    const arr = new Array(set.size);
    const values = set.values();
    for (let i = 0; i < arr.length; i++) {
        const value = values.next().value;
        arr[i] = [value, value];
    }
    return arr;
}

export { setToEntries };
