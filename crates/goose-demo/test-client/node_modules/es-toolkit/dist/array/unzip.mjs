function unzip(zipped) {
    let maxLen = 0;
    for (let i = 0; i < zipped.length; i++) {
        if (zipped[i].length > maxLen) {
            maxLen = zipped[i].length;
        }
    }
    const result = new Array(maxLen);
    for (let i = 0; i < maxLen; i++) {
        result[i] = new Array(zipped.length);
        for (let j = 0; j < zipped.length; j++) {
            result[i][j] = zipped[j][i];
        }
    }
    return result;
}

export { unzip };
