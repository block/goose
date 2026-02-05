function maxBy(items, getValue) {
    if (items.length === 0) {
        return undefined;
    }
    let maxElement = items[0];
    let max = getValue(maxElement, 0, items);
    for (let i = 1; i < items.length; i++) {
        const element = items[i];
        const value = getValue(element, i, items);
        if (value > max) {
            max = value;
            maxElement = element;
        }
    }
    return maxElement;
}

export { maxBy };
