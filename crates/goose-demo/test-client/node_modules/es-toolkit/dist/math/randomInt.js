'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const random = require('./random.js');

function randomInt(minimum, maximum) {
    return Math.floor(random.random(minimum, maximum));
}

exports.randomInt = randomInt;
