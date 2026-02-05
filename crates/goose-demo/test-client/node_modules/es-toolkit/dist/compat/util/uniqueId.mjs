let idCounter = 0;
function uniqueId(prefix = '') {
    const id = ++idCounter;
    return `${prefix}${id}`;
}

export { uniqueId };
