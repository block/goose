function round(value, precision = 0) {
    if (!Number.isInteger(precision)) {
        throw new Error('Precision must be an integer.');
    }
    const multiplier = Math.pow(10, precision);
    return Math.round(value * multiplier) / multiplier;
}

export { round };
