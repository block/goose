'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const compareValues = require('../_internal/compareValues.js');
const isKey = require('../_internal/isKey.js');
const toPath = require('../util/toPath.js');

function orderBy(collection, criteria, orders, guard) {
    if (collection == null) {
        return [];
    }
    orders = guard ? undefined : orders;
    if (!Array.isArray(collection)) {
        collection = Object.values(collection);
    }
    if (!Array.isArray(criteria)) {
        criteria = criteria == null ? [null] : [criteria];
    }
    if (criteria.length === 0) {
        criteria = [null];
    }
    if (!Array.isArray(orders)) {
        orders = orders == null ? [] : [orders];
    }
    orders = orders.map(order => String(order));
    const getValueByNestedPath = (object, path) => {
        let target = object;
        for (let i = 0; i < path.length && target != null; ++i) {
            target = target[path[i]];
        }
        return target;
    };
    const getValueByCriterion = (criterion, object) => {
        if (object == null || criterion == null) {
            return object;
        }
        if (typeof criterion === 'object' && 'key' in criterion) {
            if (Object.hasOwn(object, criterion.key)) {
                return object[criterion.key];
            }
            return getValueByNestedPath(object, criterion.path);
        }
        if (typeof criterion === 'function') {
            return criterion(object);
        }
        if (Array.isArray(criterion)) {
            return getValueByNestedPath(object, criterion);
        }
        if (typeof object === 'object') {
            return object[criterion];
        }
        return object;
    };
    const preparedCriteria = criteria.map((criterion) => {
        if (Array.isArray(criterion) && criterion.length === 1) {
            criterion = criterion[0];
        }
        if (criterion == null || typeof criterion === 'function' || Array.isArray(criterion) || isKey.isKey(criterion)) {
            return criterion;
        }
        return { key: criterion, path: toPath.toPath(criterion) };
    });
    const preparedCollection = collection.map(item => ({
        original: item,
        criteria: preparedCriteria.map((criterion) => getValueByCriterion(criterion, item)),
    }));
    return preparedCollection
        .slice()
        .sort((a, b) => {
        for (let i = 0; i < preparedCriteria.length; i++) {
            const comparedResult = compareValues.compareValues(a.criteria[i], b.criteria[i], orders[i]);
            if (comparedResult !== 0) {
                return comparedResult;
            }
        }
        return 0;
    })
        .map(item => item.original);
}

exports.orderBy = orderBy;
