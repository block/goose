function intersectionWith(firstArr, secondArr, areItemsEqual) {
    return firstArr.filter(firstItem => {
        return secondArr.some(secondItem => {
            return areItemsEqual(firstItem, secondItem);
        });
    });
}

export { intersectionWith };
