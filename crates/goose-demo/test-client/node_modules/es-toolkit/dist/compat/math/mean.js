'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sum = require('./sum.js');

function mean(nums) {
    const length = nums ? nums.length : 0;
    return length === 0 ? NaN : sum.sum(nums) / length;
}

exports.mean = mean;
