'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const timeout = require('./timeout.js');

async function withTimeout(run, ms) {
    return Promise.race([run(), timeout.timeout(ms)]);
}

exports.withTimeout = withTimeout;
