function differenceBy(firstArr, secondArr, mapper) {
    const mappedSecondSet = new Set(secondArr.map(item => mapper(item)));
    return firstArr.filter(item => {
        return !mappedSecondSet.has(mapper(item));
    });
}

export { differenceBy };
