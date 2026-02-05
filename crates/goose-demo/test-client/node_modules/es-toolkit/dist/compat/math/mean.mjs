import { sum } from './sum.mjs';

function mean(nums) {
    const length = nums ? nums.length : 0;
    return length === 0 ? NaN : sum(nums) / length;
}

export { mean };
