function sumBy(items, getValue) {
    let result = 0;
    for (let i = 0; i < items.length; i++) {
        result += getValue(items[i], i);
    }
    return result;
}

export { sumBy };
