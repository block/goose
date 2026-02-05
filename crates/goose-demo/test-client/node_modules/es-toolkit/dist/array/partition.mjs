function partition(arr, isInTruthy) {
    const truthy = [];
    const falsy = [];
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        if (isInTruthy(item, i, arr)) {
            truthy.push(item);
        }
        else {
            falsy.push(item);
        }
    }
    return [truthy, falsy];
}

export { partition };
