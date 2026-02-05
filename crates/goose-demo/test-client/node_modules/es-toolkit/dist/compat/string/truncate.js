'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isObject = require('../predicate/isObject.js');

const regexMultiByte = /[\u200d\ud800-\udfff\u0300-\u036f\ufe20-\ufe2f\u20d0-\u20ff\ufe0e\ufe0f]/;
function truncate(string, options) {
    string = string != null ? `${string}` : '';
    let length = 30;
    let omission = '...';
    if (isObject.isObject(options)) {
        length = parseLength(options.length);
        omission = 'omission' in options ? `${options.omission}` : '...';
    }
    let i = string.length;
    const lengthOmission = Array.from(omission).length;
    const lengthBase = Math.max(length - lengthOmission, 0);
    let strArray = undefined;
    const unicode = regexMultiByte.test(string);
    if (unicode) {
        strArray = Array.from(string);
        i = strArray.length;
    }
    if (length >= i) {
        return string;
    }
    if (i <= lengthOmission) {
        return omission;
    }
    let base = strArray === undefined ? string.slice(0, lengthBase) : strArray?.slice(0, lengthBase).join('');
    const separator = options?.separator;
    if (!separator) {
        base += omission;
        return base;
    }
    const search = separator instanceof RegExp ? separator.source : separator;
    const flags = 'u' + (separator instanceof RegExp ? separator.flags.replace('u', '') : '');
    const withoutSeparator = new RegExp(`(?<result>.*(?:(?!${search}).))(?:${search})`, flags).exec(base);
    return (!withoutSeparator?.groups ? base : withoutSeparator.groups.result) + omission;
}
function parseLength(length) {
    if (length == null) {
        return 30;
    }
    if (length <= 0) {
        return 0;
    }
    return length;
}

exports.truncate = truncate;
