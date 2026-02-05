'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const property = require('../object/property.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function iteratee(value) {
    if (value == null) {
        return identity.identity;
    }
    switch (typeof value) {
        case 'function': {
            return value;
        }
        case 'object': {
            if (Array.isArray(value) && value.length === 2) {
                return matchesProperty.matchesProperty(value[0], value[1]);
            }
            return matches.matches(value);
        }
        case 'string':
        case 'symbol':
        case 'number': {
            return property.property(value);
        }
    }
}

exports.iteratee = iteratee;
