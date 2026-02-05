'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isBrowser() {
    return typeof window !== 'undefined' && window?.document != null;
}

exports.isBrowser = isBrowser;
