/**
 * Converts a string to Pascal case.
 *
 * Pascal case is the naming convention in which each word is capitalized and concatenated without any separator characters.
 *
 * @param {string} str - The string that is to be changed to pascal case.
 * @returns {string} - The converted string to Pascal case.
 *
 * @example
 * const convertedStr1 = pascalCase('pascalCase') // returns 'PascalCase'
 * const convertedStr2 = pascalCase('some whitespace') // returns 'SomeWhitespace'
 * const convertedStr3 = pascalCase('hyphen-text') // returns 'HyphenText'
 * const convertedStr4 = pascalCase('HTTPRequest') // returns 'HttpRequest'
 */
declare function pascalCase(str: string): string;

export { pascalCase };
