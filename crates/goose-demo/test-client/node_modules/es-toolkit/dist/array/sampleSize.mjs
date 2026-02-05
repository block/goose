import { randomInt } from '../math/randomInt.mjs';

function sampleSize(array, size) {
    if (size > array.length) {
        throw new Error('Size must be less than or equal to the length of array.');
    }
    const result = new Array(size);
    const selected = new Set();
    for (let step = array.length - size, resultIndex = 0; step < array.length; step++, resultIndex++) {
        let index = randomInt(0, step + 1);
        if (selected.has(index)) {
            index = step;
        }
        selected.add(index);
        result[resultIndex] = array[index];
    }
    return result;
}

export { sampleSize };
