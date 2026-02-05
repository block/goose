function intersection(firstArr, secondArr) {
    const secondSet = new Set(secondArr);
    return firstArr.filter(item => secondSet.has(item));
}

export { intersection };
