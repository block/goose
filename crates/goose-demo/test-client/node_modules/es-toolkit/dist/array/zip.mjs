function zip(...arrs) {
    let rowCount = 0;
    for (let i = 0; i < arrs.length; i++) {
        if (arrs[i].length > rowCount) {
            rowCount = arrs[i].length;
        }
    }
    const columnCount = arrs.length;
    const result = Array(rowCount);
    for (let i = 0; i < rowCount; ++i) {
        const row = Array(columnCount);
        for (let j = 0; j < columnCount; ++j) {
            row[j] = arrs[j][i];
        }
        result[i] = row;
    }
    return result;
}

export { zip };
