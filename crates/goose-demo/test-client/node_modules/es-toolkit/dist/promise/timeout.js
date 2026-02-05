'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const delay = require('./delay.js');
const TimeoutError = require('../error/TimeoutError.js');

async function timeout(ms) {
    await delay.delay(ms);
    throw new TimeoutError.TimeoutError();
}

exports.timeout = timeout;
