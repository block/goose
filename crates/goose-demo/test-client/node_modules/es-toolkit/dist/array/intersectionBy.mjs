function intersectionBy(firstArr, secondArr, mapper) {
    const result = [];
    const mappedSecondSet = new Set(secondArr.map(mapper));
    for (let i = 0; i < firstArr.length; i++) {
        const item = firstArr[i];
        const mappedItem = mapper(item);
        if (mappedSecondSet.has(mappedItem)) {
            result.push(item);
            mappedSecondSet.delete(mappedItem);
        }
    }
    return result;
}

export { intersectionBy };
