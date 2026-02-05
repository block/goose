'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flattenDepth = require('./flattenDepth.js');
const isIndex = require('../_internal/isIndex.js');
const isKey = require('../_internal/isKey.js');
const toKey = require('../_internal/toKey.js');
const at = require('../object/at.js');
const unset = require('../object/unset.js');
const isArray = require('../predicate/isArray.js');
const toPath = require('../util/toPath.js');

function pullAt(array, ..._indices) {
    const indices = flattenDepth.flattenDepth(_indices, 1);
    if (!array) {
        return Array(indices.length);
    }
    const result = at.at(array, indices);
    const indicesToPull = indices
        .map(index => (isIndex.isIndex(index, array.length) ? Number(index) : index))
        .sort((a, b) => b - a);
    for (const index of new Set(indicesToPull)) {
        if (isIndex.isIndex(index, array.length)) {
            Array.prototype.splice.call(array, index, 1);
            continue;
        }
        if (isKey.isKey(index, array)) {
            delete array[toKey.toKey(index)];
            continue;
        }
        const path = isArray.isArray(index) ? index : toPath.toPath(index);
        unset.unset(array, path);
    }
    return result;
}

exports.pullAt = pullAt;
