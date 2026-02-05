function parseInt(string, radix = 0, guard) {
    if (guard) {
        radix = 0;
    }
    return Number.parseInt(string, radix);
}

export { parseInt };
