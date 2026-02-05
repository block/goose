'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sum = require('./sum.js');

function mean(nums) {
    return sum.sum(nums) / nums.length;
}

exports.mean = mean;
