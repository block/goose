import tabsToSpaces from 'convert-to-spaces';
const generateLineNumbers = (line, around) => {
    const lineNumbers = [];
    const min = line - around;
    const max = line + around;
    for (let lineNumber = min; lineNumber <= max; lineNumber++) {
        lineNumbers.push(lineNumber);
    }
    return lineNumbers;
};
const codeExcerpt = (source, line, options = {}) => {
    var _a;
    if (typeof source !== 'string') {
        throw new TypeError('Source code is missing.');
    }
    if (!line || line < 1) {
        throw new TypeError('Line number must start from `1`.');
    }
    const lines = tabsToSpaces(source).split(/\r?\n/);
    if (line > lines.length) {
        return;
    }
    return generateLineNumbers(line, (_a = options.around) !== null && _a !== void 0 ? _a : 3)
        .filter(line => lines[line - 1] !== undefined)
        .map(line => ({ line, value: lines[line - 1] }));
};
export default codeExcerpt;
