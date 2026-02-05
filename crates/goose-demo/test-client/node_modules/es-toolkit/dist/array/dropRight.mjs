function dropRight(arr, itemsCount) {
    itemsCount = Math.min(-itemsCount, 0);
    if (itemsCount === 0) {
        return arr.slice();
    }
    return arr.slice(0, itemsCount);
}

export { dropRight };
