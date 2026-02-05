'use strict';

function copyArray(source, array) {
    const length = source.length;
    if (array == null) {
        array = Array(length);
    }
    for (let i = 0; i < length; i++) {
        array[i] = source[i];
    }
    return array;
}

module.exports = copyArray;
