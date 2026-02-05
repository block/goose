function min(items) {
    if (!items || items.length === 0) {
        return undefined;
    }
    let minResult = undefined;
    for (let i = 0; i < items.length; i++) {
        const current = items[i];
        if (current == null || Number.isNaN(current) || typeof current === 'symbol') {
            continue;
        }
        if (minResult === undefined || current < minResult) {
            minResult = current;
        }
    }
    return minResult;
}

export { min };
