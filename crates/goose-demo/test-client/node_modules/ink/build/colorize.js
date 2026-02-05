import chalk from 'chalk';
const rgbRegex = /^rgb\(\s?(\d+),\s?(\d+),\s?(\d+)\s?\)$/;
const ansiRegex = /^ansi256\(\s?(\d+)\s?\)$/;
const isNamedColor = (color) => {
    return color in chalk;
};
const colorize = (str, color, type) => {
    if (!color) {
        return str;
    }
    if (isNamedColor(color)) {
        if (type === 'foreground') {
            return chalk[color](str);
        }
        const methodName = `bg${color[0].toUpperCase() + color.slice(1)}`;
        return chalk[methodName](str);
    }
    if (color.startsWith('#')) {
        return type === 'foreground'
            ? chalk.hex(color)(str)
            : chalk.bgHex(color)(str);
    }
    if (color.startsWith('ansi256')) {
        const matches = ansiRegex.exec(color);
        if (!matches) {
            return str;
        }
        const value = Number(matches[1]);
        return type === 'foreground'
            ? chalk.ansi256(value)(str)
            : chalk.bgAnsi256(value)(str);
    }
    if (color.startsWith('rgb')) {
        const matches = rgbRegex.exec(color);
        if (!matches) {
            return str;
        }
        const firstValue = Number(matches[1]);
        const secondValue = Number(matches[2]);
        const thirdValue = Number(matches[3]);
        return type === 'foreground'
            ? chalk.rgb(firstValue, secondValue, thirdValue)(str)
            : chalk.bgRgb(firstValue, secondValue, thirdValue)(str);
    }
    return str;
};
export default colorize;
//# sourceMappingURL=colorize.js.map