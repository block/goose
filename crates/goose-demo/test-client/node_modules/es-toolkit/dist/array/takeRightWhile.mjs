function takeRightWhile(arr, shouldContinueTaking) {
    for (let i = arr.length - 1; i >= 0; i--) {
        if (!shouldContinueTaking(arr[i], i, arr)) {
            return arr.slice(i + 1);
        }
    }
    return arr.slice();
}

export { takeRightWhile };
