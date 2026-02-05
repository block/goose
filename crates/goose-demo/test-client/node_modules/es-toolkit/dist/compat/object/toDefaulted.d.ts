/**
 * Creates a new object based on the provided `object`, applying default values from the `sources` to ensure that no properties are left `undefined`.
 * It assigns default values to properties that are either `undefined` or come from `Object.prototype`.
 *
 * You can provide multiple source objects to set these default values,
 * and they will be applied in the order they are given, from left to right.
 * Once a property has been set, any later values for that property will be ignored.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * @template T - The type of the object being processed.
 * @param {T} object - The target object.
 * @returns {T} The cloned object.
 */
declare function toDefaulted<T extends object>(object: T): T;
/**
 * Creates a new object based on the provided `object`, applying default values from the `sources` to ensure that no properties are left `undefined`.
 * It assigns default values to properties that are either `undefined` or come from `Object.prototype`.
 *
 * You can provide multiple source objects to set these default values,
 * and they will be applied in the order they are given, from left to right.
 * Once a property has been set, any later values for that property will be ignored.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * @template T - The type of the object being processed.
 * @template S - The type of the object that provides default values.
 * @param {T} object - The target object that will receive default values.
 * @param {S} source - The object that specifies the default values to apply.
 * @returns {NonNullable<T & S>} A new object that combines the target and default values, ensuring no properties are left undefined.
 */
declare function toDefaulted<T extends object, S extends object>(object: T, source: S): NonNullable<T & S>;
/**
 * Creates a new object based on the provided `object`, applying default values from the `sources` to ensure that no properties are left `undefined`.
 * It assigns default values to properties that are either `undefined` or come from `Object.prototype`.
 *
 * You can provide multiple source objects to set these default values,
 * and they will be applied in the order they are given, from left to right.
 * Once a property has been set, any later values for that property will be ignored.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * @template T - The type of the object being processed.
 * @template S1 - The type of the first object that provides default values.
 * @template S2 - The type of the second object that provides default values.
 * @param {T} object - The target object that will receive default values.
 * @param {S1} source1 - The first object that specifies the default values to apply.
 * @param {S2} source2 - The second object that specifies the default values to apply.
 * @returns {NonNullable<T & S1 & S2>} A new object that combines the target and default values, ensuring no properties are left undefined.
 */
declare function toDefaulted<T extends object, S1 extends object, S2 extends object>(object: T, source1: S1, source2: S2): NonNullable<T & S1 & S2>;
/**
 * Creates a new object based on the provided `object`, applying default values from the `sources` to ensure that no properties are left `undefined`.
 * It assigns default values to properties that are either `undefined` or come from `Object.prototype`.
 *
 * You can provide multiple source objects to set these default values,
 * and they will be applied in the order they are given, from left to right.
 * Once a property has been set, any later values for that property will be ignored.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * @template T - The type of the object being processed.
 * @template S1 - The type of the first object that provides default values.
 * @template S2 - The type of the second object that provides default values.
 * @template S3 - The type of the third object that provides default values.
 * @param {T} object - The target object that will receive default values.
 * @param {S1} source1 - The first object that specifies the default values to apply.
 * @param {S2} source2 - The second object that specifies the default values to apply.
 * @param {S3} source3 - The third object that specifies the default values to apply.
 * @returns {NonNullable<T & S1 & S2 & S3>} A new object that combines the target and default values, ensuring no properties are left undefined.
 */
declare function toDefaulted<T extends object, S1 extends object, S2 extends object, S3 extends object>(object: T, source1: S1, source2: S2, source3: S3): NonNullable<T & S1 & S2 & S3>;
/**
 * Creates a new object based on the provided `object`, applying default values from the `sources` to ensure that no properties are left `undefined`.
 * It assigns default values to properties that are either `undefined` or come from `Object.prototype`.
 *
 * You can provide multiple source objects to set these default values,
 * and they will be applied in the order they are given, from left to right.
 * Once a property has been set, any later values for that property will be ignored.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * @template T - The type of the object being processed.
 * @template S1 - The type of the first object that provides default values.
 * @template S2 - The type of the second object that provides default values.
 * @template S3 - The type of the third object that provides default values.
 * @template S4 - The type of the fourth object that provides default values.
 * @param {T} object - The target object that will receive default values.
 * @param {S1} source1 - The first object that specifies the default values to apply.
 * @param {S2} source2 - The second object that specifies the default values to apply.
 * @param {S3} source3 - The third object that specifies the default values to apply.
 * @param {S4} source4 - The fourth object that specifies the default values to apply.
 * @returns {NonNullable<T & S1 & S2 & S3 & S4>} A new object that combines the target and default values, ensuring no properties are left undefined.
 */
declare function toDefaulted<T extends object, S1 extends object, S2 extends object, S3 extends object, S4 extends object>(object: T, source1: S1, source2: S2, source3: S3, source4: S4): NonNullable<T & S1 & S2 & S3 & S4>;
/**
 * Creates a new object based on the provided `object`, applying default values from the `sources` to ensure that no properties are left `undefined`.
 * It assigns default values to properties that are either `undefined` or come from `Object.prototype`.
 *
 * You can provide multiple source objects to set these default values,
 * and they will be applied in the order they are given, from left to right.
 * Once a property has been set, any later values for that property will be ignored.
 *
 * Note: This function creates a new object. If you want to modify the `object`, use the `defaults` function instead.
 *
 * @template T - The type of the object being processed.
 * @template S - The type of the objects that provides default values.
 * @param {T} object - The target object that will receive default values.
 * @param {S[]} sources - The objects that specifies the default values to apply.
 * @returns {object} A new object that combines the target and default values, ensuring no properties are left undefined.
 *
 * @example
 * toDefaulted({ a: 1 }, { a: 2, b: 2 }, { c: 3 }); // { a: 1, b: 2, c: 3 }
 * toDefaulted({ a: 1, b: 2 }, { b: 3 }, { c: 3 }); // { a: 1, b: 2, c: 3 }
 * toDefaulted({ a: null }, { a: 1 }); // { a: null }
 * toDefaulted({ a: undefined }, { a: 1 }); // { a: 1 }
 */
declare function toDefaulted<T extends object, S extends object>(object: T, ...sources: S[]): object;

export { toDefaulted };
