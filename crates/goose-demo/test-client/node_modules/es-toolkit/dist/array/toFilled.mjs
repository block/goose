function toFilled(arr, value, start = 0, end = arr.length) {
    const length = arr.length;
    const finalStart = Math.max(start >= 0 ? start : length + start, 0);
    const finalEnd = Math.min(end >= 0 ? end : length + end, length);
    const newArr = arr.slice();
    for (let i = finalStart; i < finalEnd; i++) {
        newArr[i] = value;
    }
    return newArr;
}

export { toFilled };
