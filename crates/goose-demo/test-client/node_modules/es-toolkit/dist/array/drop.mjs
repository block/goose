function drop(arr, itemsCount) {
    itemsCount = Math.max(itemsCount, 0);
    return arr.slice(itemsCount);
}

export { drop };
