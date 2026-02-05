/**
 * Converts a string to kebab case.
 *
 * Kebab case is the naming convention in which each word is written in lowercase and separated by a dash (-) character.
 *
 * @param {string} str - The string that is to be changed to kebab case.
 * @returns {string} - The converted string to kebab case.
 *
 * @example
 * const convertedStr1 = kebabCase('camelCase') // returns 'camel-case'
 * const convertedStr2 = kebabCase('some whitespace') // returns 'some-whitespace'
 * const convertedStr3 = kebabCase('hyphen-text') // returns 'hyphen-text'
 * const convertedStr4 = kebabCase('HTTPRequest') // returns 'http-request'
 */
declare function kebabCase(str: string): string;

export { kebabCase };
