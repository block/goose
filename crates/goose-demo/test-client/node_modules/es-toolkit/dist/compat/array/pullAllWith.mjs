import copyArray from '../_internal/copyArray.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function pullAllWith(array, values, comparator) {
    if (array?.length == null || values?.length == null) {
        return array;
    }
    if (array === values) {
        values = copyArray(values);
    }
    let resultLength = 0;
    if (comparator == null) {
        comparator = (a, b) => isEqualsSameValueZero(a, b);
    }
    const valuesArray = Array.isArray(values) ? values : Array.from(values);
    const hasUndefined = valuesArray.includes(undefined);
    for (let i = 0; i < array.length; i++) {
        if (i in array) {
            const shouldRemove = valuesArray.some(value => comparator(array[i], value));
            if (!shouldRemove) {
                array[resultLength++] = array[i];
            }
            continue;
        }
        if (!hasUndefined) {
            delete array[resultLength++];
        }
    }
    array.length = resultLength;
    return array;
}

export { pullAllWith };
