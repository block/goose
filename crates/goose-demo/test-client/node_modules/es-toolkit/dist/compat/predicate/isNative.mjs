const functionToString = Function.prototype.toString;
const REGEXP_SYNTAX_CHARS = /[\\^$.*+?()[\]{}|]/g;
const IS_NATIVE_FUNCTION_REGEXP = RegExp(`^${functionToString
    .call(Object.prototype.hasOwnProperty)
    .replace(REGEXP_SYNTAX_CHARS, '\\$&')
    .replace(/hasOwnProperty|(function).*?(?=\\\()| for .+?(?=\\\])/g, '$1.*?')}$`);
function isNative(value) {
    if (typeof value !== 'function') {
        return false;
    }
    if (globalThis?.['__core-js_shared__'] != null) {
        throw new Error('Unsupported core-js use. Try https://npms.io/search?q=ponyfill.');
    }
    return IS_NATIVE_FUNCTION_REGEXP.test(functionToString.call(value));
}

export { isNative };
