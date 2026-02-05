import { at } from './at.mjs';

function pullAt(arr, indicesToRemove) {
    const removed = at(arr, indicesToRemove);
    const indices = new Set(indicesToRemove.slice().sort((x, y) => y - x));
    for (const index of indices) {
        arr.splice(index, 1);
    }
    return removed;
}

export { pullAt };
