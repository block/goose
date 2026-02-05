'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function spread(func) {
    return function (argsArr) {
        return func.apply(this, argsArr);
    };
}

exports.spread = spread;
