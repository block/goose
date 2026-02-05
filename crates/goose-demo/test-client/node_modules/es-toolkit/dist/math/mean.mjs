import { sum } from './sum.mjs';

function mean(nums) {
    return sum(nums) / nums.length;
}

export { mean };
