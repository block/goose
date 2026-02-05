'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const compareValues = require('../_internal/compareValues.js');

function orderBy(arr, criteria, orders) {
    return arr.slice().sort((a, b) => {
        const ordersLength = orders.length;
        for (let i = 0; i < criteria.length; i++) {
            const order = ordersLength > i ? orders[i] : orders[ordersLength - 1];
            const criterion = criteria[i];
            const criterionIsFunction = typeof criterion === 'function';
            const valueA = criterionIsFunction ? criterion(a) : a[criterion];
            const valueB = criterionIsFunction ? criterion(b) : b[criterion];
            const result = compareValues.compareValues(valueA, valueB, order);
            if (result !== 0) {
                return result;
            }
        }
        return 0;
    });
}

exports.orderBy = orderBy;
