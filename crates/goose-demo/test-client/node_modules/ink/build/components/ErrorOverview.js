import * as fs from 'node:fs';
import { cwd } from 'node:process';
import React from 'react';
import StackUtils from 'stack-utils';
import codeExcerpt from 'code-excerpt';
import Box from './Box.js';
import Text from './Text.js';
// Error's source file is reported as file:///home/user/file.js
// This function removes the file://[cwd] part
const cleanupPath = (path) => {
    return path?.replace(`file://${cwd()}/`, '');
};
const stackUtils = new StackUtils({
    cwd: cwd(),
    internals: StackUtils.nodeInternals(),
});
export default function ErrorOverview({ error }) {
    const stack = error.stack ? error.stack.split('\n').slice(1) : undefined;
    const origin = stack ? stackUtils.parseLine(stack[0]) : undefined;
    const filePath = cleanupPath(origin?.file);
    let excerpt;
    let lineWidth = 0;
    if (filePath && origin?.line && fs.existsSync(filePath)) {
        const sourceCode = fs.readFileSync(filePath, 'utf8');
        excerpt = codeExcerpt(sourceCode, origin.line);
        if (excerpt) {
            for (const { line } of excerpt) {
                lineWidth = Math.max(lineWidth, String(line).length);
            }
        }
    }
    return (React.createElement(Box, { flexDirection: "column", padding: 1 },
        React.createElement(Box, null,
            React.createElement(Text, { backgroundColor: "red", color: "white" },
                ' ',
                "ERROR",
                ' '),
            React.createElement(Text, null,
                " ",
                error.message)),
        origin && filePath && (React.createElement(Box, { marginTop: 1 },
            React.createElement(Text, { dimColor: true },
                filePath,
                ":",
                origin.line,
                ":",
                origin.column))),
        origin && excerpt && (React.createElement(Box, { marginTop: 1, flexDirection: "column" }, excerpt.map(({ line, value }) => (React.createElement(Box, { key: line },
            React.createElement(Box, { width: lineWidth + 1 },
                React.createElement(Text, { dimColor: line !== origin.line, backgroundColor: line === origin.line ? 'red' : undefined, color: line === origin.line ? 'white' : undefined },
                    String(line).padStart(lineWidth, ' '),
                    ":")),
            React.createElement(Text, { key: line, backgroundColor: line === origin.line ? 'red' : undefined, color: line === origin.line ? 'white' : undefined }, ' ' + value)))))),
        error.stack && (React.createElement(Box, { marginTop: 1, flexDirection: "column" }, error.stack
            .split('\n')
            .slice(1)
            .map(line => {
            const parsedLine = stackUtils.parseLine(line);
            // If the line from the stack cannot be parsed, we print out the unparsed line.
            if (!parsedLine) {
                return (React.createElement(Box, { key: line },
                    React.createElement(Text, { dimColor: true }, "- "),
                    React.createElement(Text, { dimColor: true, bold: true }, line)));
            }
            return (React.createElement(Box, { key: line },
                React.createElement(Text, { dimColor: true }, "- "),
                React.createElement(Text, { dimColor: true, bold: true }, parsedLine.function),
                React.createElement(Text, { dimColor: true, color: "gray" },
                    ' ',
                    "(",
                    cleanupPath(parsedLine.file) ?? '',
                    ":",
                    parsedLine.line,
                    ":",
                    parsedLine.column,
                    ")")));
        })))));
}
//# sourceMappingURL=ErrorOverview.js.map