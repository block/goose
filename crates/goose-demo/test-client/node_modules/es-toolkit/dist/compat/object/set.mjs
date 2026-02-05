import { updateWith } from './updateWith.mjs';

function set(obj, path, value) {
    return updateWith(obj, path, () => value, () => undefined);
}

export { set };
