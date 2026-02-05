import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

const assignValue = (object, key, value) => {
    const objValue = object[key];
    if (!(Object.hasOwn(object, key) && isEqualsSameValueZero(objValue, value)) || (value === undefined && !(key in object))) {
        object[key] = value;
    }
};

export { assignValue };
