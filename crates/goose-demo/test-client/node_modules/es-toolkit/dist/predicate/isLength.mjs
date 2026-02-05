function isLength(value) {
    return Number.isSafeInteger(value) && value >= 0;
}

export { isLength };
