/**
 * Maps each element of an array based on a provided key-generating function.
 *
 * This function takes an array and a function that generates a key from each element. It returns
 * an object where the keys are the generated keys and the values are the corresponding elements.
 * If there are multiple elements generating the same key, the last element among them is used
 * as the value.
 *
 * @template T - The type of elements in the array.
 * @template K - The type of keys.
 * @param {T[]} arr - The array of elements to be mapped.
 * @param {(item: T, index: number, array: readonly T[]) => K} getKeyFromItem - A function that generates a key from an element, its index, and the array.
 * @returns {Record<K, T>} An object where keys are mapped to each element of an array.
 *
 * @example
 * const array = [
 *   { category: 'fruit', name: 'apple' },
 *   { category: 'fruit', name: 'banana' },
 *   { category: 'vegetable', name: 'carrot' }
 * ];
 * const result = keyBy(array, item => item.category);
 * // result will be:
 * // {
 * //   fruit: { category: 'fruit', name: 'banana' },
 * //   vegetable: { category: 'vegetable', name: 'carrot' }
 * // }
 *
 * @example
 * // Using index parameter
 * const items = ['a', 'b', 'c'];
 * const result = keyBy(items, (item, index) => index);
 * // result will be: { 0: 'a', 1: 'b', 2: 'c' }
 */
declare function keyBy<T, K extends PropertyKey>(arr: readonly T[], getKeyFromItem: (item: T, index: number, array: readonly T[]) => K): Record<K, T>;

export { keyBy };
