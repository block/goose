import { updateWith } from './updateWith.mjs';

function setWith(obj, path, value, customizer) {
    let customizerFn;
    if (typeof customizer === 'function') {
        customizerFn = customizer;
    }
    else {
        customizerFn = () => undefined;
    }
    return updateWith(obj, path, () => value, customizerFn);
}

export { setWith };
