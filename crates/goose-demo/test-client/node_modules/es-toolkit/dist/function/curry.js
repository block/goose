'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function curry(func) {
    if (func.length === 0 || func.length === 1) {
        return func;
    }
    return function (arg) {
        return makeCurry(func, func.length, [arg]);
    };
}
function makeCurry(origin, argsLength, args) {
    if (args.length === argsLength) {
        return origin(...args);
    }
    else {
        const next = function (arg) {
            return makeCurry(origin, argsLength, [...args, arg]);
        };
        return next;
    }
}

exports.curry = curry;
