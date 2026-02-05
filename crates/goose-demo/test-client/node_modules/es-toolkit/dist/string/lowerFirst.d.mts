/**
 * Converts the first character of string to lower case.
 *
 * @param {string} str - The string that is to be changed
 * @returns {string} - The converted string.
 *
 * @example
 * const convertedStr1 = lowerCase('fred') // returns 'fred'
 * const convertedStr2 = lowerCase('Fred') // returns 'fred'
 * const convertedStr3 = lowerCase('FRED') // returns 'fRED'
 */
declare function lowerFirst(str: string): string;

export { lowerFirst };
