'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const iteratee = require('./iteratee.js');

function over(...iteratees) {
    if (iteratees.length === 1 && Array.isArray(iteratees[0])) {
        iteratees = iteratees[0];
    }
    const funcs = iteratees.map(item => iteratee.iteratee(item));
    return function (...args) {
        return funcs.map(func => func.apply(this, args));
    };
}

exports.over = over;
