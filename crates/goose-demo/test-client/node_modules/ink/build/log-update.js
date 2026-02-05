import ansiEscapes from 'ansi-escapes';
import cliCursor from 'cli-cursor';
const create = (stream, { showCursor = false } = {}) => {
    let previousLineCount = 0;
    let previousOutput = '';
    let hasHiddenCursor = false;
    const render = (str) => {
        if (!showCursor && !hasHiddenCursor) {
            cliCursor.hide();
            hasHiddenCursor = true;
        }
        const output = str + '\n';
        if (output === previousOutput) {
            return;
        }
        previousOutput = output;
        stream.write(ansiEscapes.eraseLines(previousLineCount) + output);
        previousLineCount = output.split('\n').length;
    };
    render.clear = () => {
        stream.write(ansiEscapes.eraseLines(previousLineCount));
        previousOutput = '';
        previousLineCount = 0;
    };
    render.done = () => {
        previousOutput = '';
        previousLineCount = 0;
        if (!showCursor) {
            cliCursor.show();
            hasHiddenCursor = false;
        }
    };
    return render;
};
const logUpdate = { create };
export default logUpdate;
//# sourceMappingURL=log-update.js.map