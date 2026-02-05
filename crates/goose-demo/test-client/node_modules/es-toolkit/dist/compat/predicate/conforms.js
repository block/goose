'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const conformsTo = require('./conformsTo.js');
const cloneDeep = require('../../object/cloneDeep.js');

function conforms(source) {
    source = cloneDeep.cloneDeep(source);
    return function (object) {
        return conformsTo.conformsTo(object, source);
    };
}

exports.conforms = conforms;
