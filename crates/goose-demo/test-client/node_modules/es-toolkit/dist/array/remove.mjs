function remove(arr, shouldRemoveElement) {
    const originalArr = arr.slice();
    const removed = [];
    let resultIndex = 0;
    for (let i = 0; i < arr.length; i++) {
        if (shouldRemoveElement(arr[i], i, originalArr)) {
            removed.push(arr[i]);
            continue;
        }
        if (!Object.hasOwn(arr, i)) {
            delete arr[resultIndex++];
            continue;
        }
        arr[resultIndex++] = arr[i];
    }
    arr.length = resultIndex;
    return removed;
}

export { remove };
