'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isMatch = require('./isMatch.js');
const toKey = require('../_internal/toKey.js');
const cloneDeep = require('../object/cloneDeep.js');
const get = require('../object/get.js');
const has = require('../object/has.js');

function matchesProperty(property, source) {
    switch (typeof property) {
        case 'object': {
            if (Object.is(property?.valueOf(), -0)) {
                property = '-0';
            }
            break;
        }
        case 'number': {
            property = toKey.toKey(property);
            break;
        }
    }
    source = cloneDeep.cloneDeep(source);
    return function (target) {
        const result = get.get(target, property);
        if (result === undefined) {
            return has.has(target, property);
        }
        if (source === undefined) {
            return result === undefined;
        }
        return isMatch.isMatch(result, source);
    };
}

exports.matchesProperty = matchesProperty;
