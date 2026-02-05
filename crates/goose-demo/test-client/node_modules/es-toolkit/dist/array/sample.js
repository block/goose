'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function sample(arr) {
    const randomIndex = Math.floor(Math.random() * arr.length);
    return arr[randomIndex];
}

exports.sample = sample;
