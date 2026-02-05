'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const mapValues$1 = require('../../object/mapValues.js');
const iteratee = require('../util/iteratee.js');

function mapValues(object, getNewValue = identity.identity) {
    if (object == null) {
        return {};
    }
    return mapValues$1.mapValues(object, iteratee.iteratee(getNewValue));
}

exports.mapValues = mapValues;
