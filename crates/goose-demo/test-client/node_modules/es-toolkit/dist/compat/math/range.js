'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isIterateeCall = require('../_internal/isIterateeCall.js');
const toFinite = require('../util/toFinite.js');

function range(start, end, step) {
    if (step && typeof step !== 'number' && isIterateeCall.isIterateeCall(start, end, step)) {
        end = step = undefined;
    }
    start = toFinite.toFinite(start);
    if (end === undefined) {
        end = start;
        start = 0;
    }
    else {
        end = toFinite.toFinite(end);
    }
    step = step === undefined ? (start < end ? 1 : -1) : toFinite.toFinite(step);
    const length = Math.max(Math.ceil((end - start) / (step || 1)), 0);
    const result = new Array(length);
    for (let index = 0; index < length; index++) {
        result[index] = start;
        start += step;
    }
    return result;
}

exports.range = range;
