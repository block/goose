import { isFunction } from '../../predicate/isFunction.mjs';
import { isArray } from '../predicate/isArray.mjs';
import { isObject } from '../predicate/isObject.mjs';
import { toString } from './toString.mjs';

function bindAll(object, ...methodNames) {
    if (object == null) {
        return object;
    }
    if (!isObject(object)) {
        return object;
    }
    if (isArray(object) && methodNames.length === 0) {
        return object;
    }
    const methods = [];
    for (let i = 0; i < methodNames.length; i++) {
        const name = methodNames[i];
        if (isArray(name)) {
            methods.push(...name);
        }
        else if (name && typeof name === 'object' && 'length' in name) {
            methods.push(...Array.from(name));
        }
        else {
            methods.push(name);
        }
    }
    if (methods.length === 0) {
        return object;
    }
    for (let i = 0; i < methods.length; i++) {
        const key = methods[i];
        const stringKey = toString(key);
        const func = object[stringKey];
        if (isFunction(func)) {
            object[stringKey] = func.bind(object);
        }
    }
    return object;
}

export { bindAll };
