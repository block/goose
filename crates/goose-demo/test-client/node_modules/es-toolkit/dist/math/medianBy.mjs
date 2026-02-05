import { median } from './median.mjs';

function medianBy(items, getValue) {
    const nums = items.map(x => getValue(x));
    return median(nums);
}

export { medianBy };
