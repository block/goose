import { PassThrough } from 'node:stream';
const consoleMethods = [
    'assert',
    'count',
    'countReset',
    'debug',
    'dir',
    'dirxml',
    'error',
    'group',
    'groupCollapsed',
    'groupEnd',
    'info',
    'log',
    'table',
    'time',
    'timeEnd',
    'timeLog',
    'trace',
    'warn',
];
let originalMethods = {};
const patchConsole = (callback) => {
    const stdout = new PassThrough();
    const stderr = new PassThrough();
    stdout.write = (data) => {
        callback('stdout', data);
    };
    stderr.write = (data) => {
        callback('stderr', data);
    };
    const internalConsole = new console.Console(stdout, stderr);
    for (const method of consoleMethods) {
        originalMethods[method] = console[method];
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        console[method] = internalConsole[method];
    }
    return () => {
        for (const method of consoleMethods) {
            console[method] = originalMethods[method];
        }
        originalMethods = {};
    };
};
export default patchConsole;
