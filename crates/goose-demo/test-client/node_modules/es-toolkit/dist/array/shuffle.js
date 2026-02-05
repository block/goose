'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function shuffle(arr) {
    const result = arr.slice();
    for (let i = result.length - 1; i >= 1; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [result[i], result[j]] = [result[j], result[i]];
    }
    return result;
}

exports.shuffle = shuffle;
