import { updateWith } from './updateWith.mjs';

function update(obj, path, updater) {
    return updateWith(obj, path, updater, () => undefined);
}

export { update };
