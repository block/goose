import { keysIn } from '../object/keysIn.mjs';

function toPlainObject(value) {
    const plainObject = {};
    const valueKeys = keysIn(value);
    for (let i = 0; i < valueKeys.length; i++) {
        const key = valueKeys[i];
        const objValue = value[key];
        if (key === '__proto__') {
            Object.defineProperty(plainObject, key, {
                configurable: true,
                enumerable: true,
                value: objValue,
                writable: true,
            });
        }
        else {
            plainObject[key] = objValue;
        }
    }
    return plainObject;
}

export { toPlainObject };
