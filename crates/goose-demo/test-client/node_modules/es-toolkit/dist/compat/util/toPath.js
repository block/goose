'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('./toString.js');
const toKey = require('../_internal/toKey.js');

function toPath(deepKey) {
    if (Array.isArray(deepKey)) {
        return deepKey.map(toKey.toKey);
    }
    if (typeof deepKey === 'symbol') {
        return [deepKey];
    }
    deepKey = toString.toString(deepKey);
    const result = [];
    const length = deepKey.length;
    if (length === 0) {
        return result;
    }
    let index = 0;
    let key = '';
    let quoteChar = '';
    let bracket = false;
    if (deepKey.charCodeAt(0) === 46) {
        result.push('');
        index++;
    }
    while (index < length) {
        const char = deepKey[index];
        if (quoteChar) {
            if (char === '\\' && index + 1 < length) {
                index++;
                key += deepKey[index];
            }
            else if (char === quoteChar) {
                quoteChar = '';
            }
            else {
                key += char;
            }
        }
        else if (bracket) {
            if (char === '"' || char === "'") {
                quoteChar = char;
            }
            else if (char === ']') {
                bracket = false;
                result.push(key);
                key = '';
            }
            else {
                key += char;
            }
        }
        else {
            if (char === '[') {
                bracket = true;
                if (key) {
                    result.push(key);
                    key = '';
                }
            }
            else if (char === '.') {
                if (key) {
                    result.push(key);
                    key = '';
                }
            }
            else {
                key += char;
            }
        }
        index++;
    }
    if (key) {
        result.push(key);
    }
    return result;
}

exports.toPath = toPath;
