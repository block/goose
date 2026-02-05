type SnakeToCamel<S extends string> = S extends `${infer H}_${infer T}` ? `${Lowercase<H>}${Capitalize<SnakeToCamel<T>>}` : Lowercase<S>;
type PascalToCamel<S extends string> = S extends `${infer F}${infer R}` ? `${Lowercase<F>}${R}` : S;
/** If it's snake_case, apply the snake_case rule; for uppercase keys, lowercase the entire string; otherwise, just lowercase the first letter (including PascalCase → camelCase). */
type AnyToCamel<S extends string> = S extends `${string}_${string}` ? SnakeToCamel<S> : S extends Uppercase<S> ? Lowercase<S> : PascalToCamel<S>;
type NonPlainObject = Date | RegExp | Map<any, any> | Set<any> | WeakMap<any, any> | WeakSet<any> | Promise<any> | Error | ArrayBuffer | DataView | Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | Float32Array | Float64Array | BigInt64Array | BigUint64Array | ((...args: any[]) => any) | typeof globalThis;
type ToCamelCaseKeys<T> = T extends NonPlainObject ? T : T extends any[] ? Array<ToCamelCaseKeys<T[number]>> : T extends Record<string, any> ? {
    [K in keyof T as AnyToCamel<Extract<K, string>>]: ToCamelCaseKeys<T[K]>;
} : T;
/**
 * Creates a new object composed of the properties with keys converted to camelCase.
 *
 * This function takes an object and returns a new object that includes the same properties,
 * but with all keys converted to camelCase format.
 *
 * @template T - The type of object.
 * @param {T} obj - The object to convert keys from.
 * @returns {ToCamelCaseKeys<T>} A new object with all keys converted to camelCase.
 *
 * @example
 * // Example with objects
 * const obj = { user_id: 1, first_name: 'John' };
 * const result = toCamelCaseKeys(obj);
 * // result will be { userId: 1, firstName: 'John' }
 *
 * // Example with arrays of objects
 * const arr = [
 *   { user_id: 1, first_name: 'John' },
 *   { user_id: 2, first_name: 'Jane' }
 * ];
 * const arrResult = toCamelCaseKeys(arr);
 * // arrResult will be [{ userId: 1, firstName: 'John' }, { userId: 2, firstName: 'Jane' }]
 *
 * // Example with nested objects
 * const nested = {
 *   user_data: {
 *     user_id: 1,
 *     user_address: {
 *       street_name: 'Main St',
 *       zip_code: '12345'
 *     }
 *   }
 * };
 * const nestedResult = toCamelCaseKeys(nested);
 * // nestedResult will be:
 * // {
 * //   userData: {
 * //     userId: 1,
 * //     userAddress: {
 * //       streetName: 'Main St',
 * //       zipCode: '12345'
 * //     }
 * //   }
 * // }
 */
declare function toCamelCaseKeys<T>(obj: T): ToCamelCaseKeys<T>;

export { toCamelCaseKeys };
