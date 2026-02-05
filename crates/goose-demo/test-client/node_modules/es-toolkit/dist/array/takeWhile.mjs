function takeWhile(arr, shouldContinueTaking) {
    const result = [];
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        if (!shouldContinueTaking(item, i, arr)) {
            break;
        }
        result.push(item);
    }
    return result;
}

export { takeWhile };
