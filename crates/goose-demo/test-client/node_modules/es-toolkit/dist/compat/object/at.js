'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const get = require('./get.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isString = require('../predicate/isString.js');

function at(object, ...paths) {
    if (paths.length === 0) {
        return [];
    }
    const allPaths = [];
    for (let i = 0; i < paths.length; i++) {
        const path = paths[i];
        if (!isArrayLike.isArrayLike(path) || isString.isString(path)) {
            allPaths.push(path);
            continue;
        }
        for (let j = 0; j < path.length; j++) {
            allPaths.push(path[j]);
        }
    }
    const result = [];
    for (let i = 0; i < allPaths.length; i++) {
        result.push(get.get(object, allPaths[i]));
    }
    return result;
}

exports.at = at;
