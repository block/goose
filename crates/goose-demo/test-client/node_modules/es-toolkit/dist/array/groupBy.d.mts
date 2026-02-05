/**
 * Groups the elements of an array based on a provided key-generating function.
 *
 * This function takes an array and a function that generates a key from each element. It returns
 * an object where the keys are the generated keys and the values are arrays of elements that share
 * the same key.
 *
 * @template T - The type of elements in the array.
 * @template K - The type of keys.
 * @param {T[]} arr - The array to group.
 * @param {(item: T, index: number, array: readonly T[]) => K} getKeyFromItem - A function that generates a key from an element, its index, and the array.
 * @returns {Record<K, T[]>} An object where each key is associated with an array of elements that
 * share that key.
 *
 * @example
 * const array = [
 *   { category: 'fruit', name: 'apple' },
 *   { category: 'fruit', name: 'banana' },
 *   { category: 'vegetable', name: 'carrot' }
 * ];
 * const result = groupBy(array, item => item.category);
 * // result will be:
 * // {
 * //   fruit: [
 * //     { category: 'fruit', name: 'apple' },
 * //     { category: 'fruit', name: 'banana' }
 * //   ],
 * //   vegetable: [
 * //     { category: 'vegetable', name: 'carrot' }
 * //   ]
 * // }
 *
 * @example
 * // Using index parameter
 * const items = ['a', 'b', 'c', 'd'];
 * const result = groupBy(items, (item, index) => index % 2 === 0 ? 'even' : 'odd');
 * // result will be: { even: ['a', 'c'], odd: ['b', 'd'] }
 */
declare function groupBy<T, K extends PropertyKey>(arr: readonly T[], getKeyFromItem: (item: T, index: number, array: readonly T[]) => K): Record<K, T[]>;

export { groupBy };
