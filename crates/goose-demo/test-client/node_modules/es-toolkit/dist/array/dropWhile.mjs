function dropWhile(arr, canContinueDropping) {
    const dropEndIndex = arr.findIndex((item, index, arr) => !canContinueDropping(item, index, arr));
    if (dropEndIndex === -1) {
        return [];
    }
    return arr.slice(dropEndIndex);
}

export { dropWhile };
