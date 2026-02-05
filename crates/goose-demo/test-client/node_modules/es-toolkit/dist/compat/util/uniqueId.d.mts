/**
 * Generates a unique identifier, optionally prefixed with a given string.
 *
 * @param {string} [prefix] - An optional string to prefix the unique identifier.
 *                            If not provided or not a string, only the unique
 *                            numeric identifier is returned.
 * @returns {string} A string containing the unique identifier, with the optional
 *                   prefix if provided.
 *
 * @example
 * // Generate a unique ID with a prefix
 * uniqueId('user_');  // => 'user_1'
 *
 * @example
 * // Generate a unique ID without a prefix
 * uniqueId();  // => '2'
 *
 * @example
 * // Subsequent calls increment the internal counter
 * uniqueId('item_');  // => 'item_3'
 * uniqueId();         // => '4'
 */
declare function uniqueId(prefix?: string): string;

export { uniqueId };
