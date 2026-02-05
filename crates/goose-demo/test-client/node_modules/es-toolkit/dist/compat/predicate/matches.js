'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isMatch = require('./isMatch.js');
const cloneDeep = require('../../object/cloneDeep.js');

function matches(source) {
    source = cloneDeep.cloneDeep(source);
    return (target) => {
        return isMatch.isMatch(target, source);
    };
}

exports.matches = matches;
