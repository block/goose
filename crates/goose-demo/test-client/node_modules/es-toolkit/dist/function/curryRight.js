'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function curryRight(func) {
    if (func.length === 0 || func.length === 1) {
        return func;
    }
    return function (arg) {
        return makeCurryRight(func, func.length, [arg]);
    };
}
function makeCurryRight(origin, argsLength, args) {
    if (args.length === argsLength) {
        return origin(...args);
    }
    else {
        const next = function (arg) {
            return makeCurryRight(origin, argsLength, [arg, ...args]);
        };
        return next;
    }
}

exports.curryRight = curryRight;
