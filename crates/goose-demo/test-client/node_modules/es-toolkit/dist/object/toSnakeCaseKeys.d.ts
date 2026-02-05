type SnakeCase<S extends string> = S extends Uppercase<S> ? Lowercase<S> : S extends `${infer P1}${infer P2}` ? P2 extends Uncapitalize<P2> ? `${Lowercase<P1>}${SnakeCase<P2>}` : `${Lowercase<P1>}_${SnakeCase<Uncapitalize<P2>>}` : Lowercase<S>;
type NonPlainObject = Date | RegExp | Map<any, any> | Set<any> | WeakMap<any, any> | WeakSet<any> | Promise<any> | Error | ArrayBuffer | DataView | Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | Float32Array | Float64Array | BigInt64Array | BigUint64Array | ((...args: any[]) => any) | typeof globalThis;
type ToSnakeCaseKeys<T> = T extends NonPlainObject ? T : T extends any[] ? Array<ToSnakeCaseKeys<T[number]>> : T extends Record<string, any> ? {
    [K in keyof T as SnakeCase<string & K>]: ToSnakeCaseKeys<T[K]>;
} : T;
/**
 * Creates a new object composed of the properties with keys converted to snake_case.
 *
 * This function takes an object and returns a new object that includes the same properties,
 * but with all keys converted to snake_case format.
 *
 * @template T - The type of object.
 * @param {T} obj - The object to convert keys from.
 * @returns {ToSnakeCaseKeys<T>} A new object with all keys converted to snake_case.
 *
 * @example
 * // Example with objects
 * const obj = { userId: 1, firstName: 'John' };
 * const result = toSnakeCaseKeys(obj);
 * // result will be { user_id: 1, first_name: 'John' }
 *
 * // Example with arrays of objects
 * const arr = [
 *   { userId: 1, firstName: 'John' },
 *   { userId: 2, firstName: 'Jane' }
 * ];
 * const arrResult = toSnakeCaseKeys(arr);
 * // arrResult will be [{ user_id: 1, first_name: 'John' }, { user_id: 2, first_name: 'Jane' }]
 *
 * // Example with nested objects
 * const nested = {
 *   userData: {
 *     userId: 1,
 *     userAddress: {
 *       streetName: 'Main St',
 *       zipCode: '12345'
 *     }
 *   }
 * };
 * const nestedResult = toSnakeCaseKeys(nested);
 * // nestedResult will be:
 * // {
 * //   user_data: {
 * //     user_id: 1,
 * //     user_address: {
 * //       street_name: 'Main St',
 * //       zip_code: '12345'
 * //     }
 * //   }
 * // }
 */
declare function toSnakeCaseKeys<T>(obj: T): ToSnakeCaseKeys<T>;

export { type ToSnakeCaseKeys, toSnakeCaseKeys };
