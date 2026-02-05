/**
 * Inverts the keys and values of an object. The keys of the input object become the values of the output object and vice versa.
 *
 * This function takes an object and creates a new object by inverting its keys and values. If the input object has duplicate values,
 * the key of the last occurrence will be used as the value for the new key in the output object. It effectively creates a reverse mapping
 * of the input object's key-value pairs.
 *
 * @template K - Type of the keys in the input object (string, number, symbol)
 * @template V - Type of the values in the input object (string, number, symbol)
 * @param {Record<K, V>} obj - The input object whose keys and values are to be inverted
 * @returns {Record<V, K>} - A new object with keys and values inverted
 *
 * @example
 * invert({ a: 1, b: 2, c: 3 }); // { 1: 'a', 2: 'b', 3: 'c' }
 * invert({ 1: 'a', 2: 'b', 3: 'c' }); // { a: '1', b: '2', c: '3' }
 * invert({ a: 1, 2: 'b', c: 3, 4: 'd' }); // { 1: 'a', b: '2', 3: 'c', d: '4' }
 * invert({ a: Symbol('sym1'), b: Symbol('sym2') }); // { [Symbol('sym1')]: 'a', [Symbol('sym2')]: 'b' }
 */
declare function invert<K extends PropertyKey, V extends PropertyKey>(obj: Record<K, V>): Record<V, K>;

export { invert };
