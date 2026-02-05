/**
 * Converts a string to lower case.
 *
 * Lower case is the naming convention in which each word is written in lowercase and separated by an space ( ) character.
 *
 * @param {string} str - The string that is to be changed to lower case.
 * @returns {string} - The converted string to lower case.
 *
 * @example
 * const convertedStr1 = lowerCase('camelCase') // returns 'camel case'
 * const convertedStr2 = lowerCase('some whitespace') // returns 'some whitespace'
 * const convertedStr3 = lowerCase('hyphen-text') // returns 'hyphen text'
 * const convertedStr4 = lowerCase('HTTPRequest') // returns 'http request'
 */
declare function lowerCase(str: string): string;

export { lowerCase };
