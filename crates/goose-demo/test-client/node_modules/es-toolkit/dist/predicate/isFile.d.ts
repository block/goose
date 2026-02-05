/**
 * Checks if the given value is a File.
 *
 * This function tests whether the provided value is an instance of `File`.
 * It returns `true` if the value is an instance of `File`, and `false` otherwise.
 *
 * @param {unknown} x - The value to test if it is a File.
 * @returns {x is File} True if the value is a File, false otherwise.
 *
 * @example
 * const value1 = new File(["content"], "example.txt");
 * const value2 = {};
 * const value3 = new Blob(["content"], { type: "text/plain" });
 *
 * console.log(isFile(value1)); // true
 * console.log(isFile(value2)); // false
 * console.log(isFile(value3)); // false
 */
declare function isFile(x: unknown): x is File;

export { isFile };
