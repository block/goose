'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const property = require('../object/property.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function some(source, predicate, guard) {
    if (!source) {
        return false;
    }
    if (guard != null) {
        predicate = undefined;
    }
    if (predicate == null) {
        predicate = identity.identity;
    }
    const values = Array.isArray(source) ? source : Object.values(source);
    switch (typeof predicate) {
        case 'function': {
            if (!Array.isArray(source)) {
                const keys = Object.keys(source);
                for (let i = 0; i < keys.length; i++) {
                    const key = keys[i];
                    const value = source[key];
                    if (predicate(value, key, source)) {
                        return true;
                    }
                }
                return false;
            }
            for (let i = 0; i < source.length; i++) {
                if (predicate(source[i], i, source)) {
                    return true;
                }
            }
            return false;
        }
        case 'object': {
            if (Array.isArray(predicate) && predicate.length === 2) {
                const key = predicate[0];
                const value = predicate[1];
                const matchFunc = matchesProperty.matchesProperty(key, value);
                if (Array.isArray(source)) {
                    for (let i = 0; i < source.length; i++) {
                        if (matchFunc(source[i])) {
                            return true;
                        }
                    }
                    return false;
                }
                return values.some(matchFunc);
            }
            else {
                const matchFunc = matches.matches(predicate);
                if (Array.isArray(source)) {
                    for (let i = 0; i < source.length; i++) {
                        if (matchFunc(source[i])) {
                            return true;
                        }
                    }
                    return false;
                }
                return values.some(matchFunc);
            }
        }
        case 'number':
        case 'symbol':
        case 'string': {
            const propFunc = property.property(predicate);
            if (Array.isArray(source)) {
                for (let i = 0; i < source.length; i++) {
                    if (propFunc(source[i])) {
                        return true;
                    }
                }
                return false;
            }
            return values.some(propFunc);
        }
    }
}

exports.some = some;
