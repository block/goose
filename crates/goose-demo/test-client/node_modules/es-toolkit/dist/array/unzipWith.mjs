function unzipWith(target, iteratee) {
    const maxLength = Math.max(...target.map(innerArray => innerArray.length));
    const result = new Array(maxLength);
    for (let i = 0; i < maxLength; i++) {
        const group = new Array(target.length);
        for (let j = 0; j < target.length; j++) {
            group[j] = target[j][i];
        }
        result[i] = iteratee(...group);
    }
    return result;
}

export { unzipWith };
