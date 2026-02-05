import wrapAnsi from 'wrap-ansi';
import cliTruncate from 'cli-truncate';
const cache = {};
const wrapText = (text, maxWidth, wrapType) => {
    const cacheKey = text + String(maxWidth) + String(wrapType);
    const cachedText = cache[cacheKey];
    if (cachedText) {
        return cachedText;
    }
    let wrappedText = text;
    if (wrapType === 'wrap') {
        wrappedText = wrapAnsi(text, maxWidth, {
            trim: false,
            hard: true,
        });
    }
    if (wrapType.startsWith('truncate')) {
        let position = 'end';
        if (wrapType === 'truncate-middle') {
            position = 'middle';
        }
        if (wrapType === 'truncate-start') {
            position = 'start';
        }
        wrappedText = cliTruncate(text, maxWidth, { position });
    }
    cache[cacheKey] = wrappedText;
    return wrappedText;
};
export default wrapText;
//# sourceMappingURL=wrap-text.js.map