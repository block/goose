function isJSON(value) {
    if (typeof value !== 'string') {
        return false;
    }
    try {
        JSON.parse(value);
        return true;
    }
    catch {
        return false;
    }
}

export { isJSON };
