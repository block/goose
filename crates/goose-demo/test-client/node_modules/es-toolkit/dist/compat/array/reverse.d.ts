import { MutableList } from '../_internal/MutableList.d.js';
import { RejectReadonly } from '../_internal/RejectReadonly.d.js';

/**
 * Reverses `array` so that the first element becomes the last, the second element becomes the second to last, and so on.
 *
 * @template L
 * @param {L extends readonly any[] ? never : L} array - The array to reverse.
 * @returns {L} Returns `array`.
 *
 * @example
 * const array = [1, 2, 3];
 * reverse(array);
 * // => [3, 2, 1]
 */
declare function reverse<L extends MutableList<any>>(array: RejectReadonly<L>): L;

export { reverse };
