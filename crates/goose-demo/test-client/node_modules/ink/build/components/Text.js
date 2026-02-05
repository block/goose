import React from 'react';
import chalk from 'chalk';
import colorize from '../colorize.js';
/**
 * This component can display text, and change its style to make it colorful, bold, underline, italic or strikethrough.
 */
export default function Text({ color, backgroundColor, dimColor = false, bold = false, italic = false, underline = false, strikethrough = false, inverse = false, wrap = 'wrap', children, }) {
    if (children === undefined || children === null) {
        return null;
    }
    const transform = (children) => {
        if (dimColor) {
            children = chalk.dim(children);
        }
        if (color) {
            children = colorize(children, color, 'foreground');
        }
        if (backgroundColor) {
            children = colorize(children, backgroundColor, 'background');
        }
        if (bold) {
            children = chalk.bold(children);
        }
        if (italic) {
            children = chalk.italic(children);
        }
        if (underline) {
            children = chalk.underline(children);
        }
        if (strikethrough) {
            children = chalk.strikethrough(children);
        }
        if (inverse) {
            children = chalk.inverse(children);
        }
        return children;
    };
    return (React.createElement("ink-text", { style: { flexGrow: 0, flexShrink: 1, flexDirection: 'row', textWrap: wrap }, internal_transform: transform }, children));
}
//# sourceMappingURL=Text.js.map