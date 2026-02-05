/**
 * Converts a string to constant case.
 *
 * Constant case is a naming convention where each word is written in uppercase letters and separated by an underscore (`_`). For example, `CONSTANT_CASE`.
 *
 * @param {string} str - The string that is to be changed to constant case.
 * @returns {string} - The converted string to constant case.
 *
 * @example
 * const convertedStr1 = constantCase('camelCase') // returns 'CAMEL_CASE'
 * const convertedStr2 = constantCase('some whitespace') // returns 'SOME_WHITESPACE'
 * const convertedStr3 = constantCase('hyphen-text') // returns 'HYPHEN_TEXT'
 * const convertedStr4 = constantCase('HTTPRequest') // returns 'HTTP_REQUEST'
 */
declare function constantCase(str: string): string;

export { constantCase };
