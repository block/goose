'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function partialRight(func, ...partialArgs) {
    return partialRightImpl(func, placeholderSymbol, ...partialArgs);
}
function partialRightImpl(func, placeholder, ...partialArgs) {
    const partialedRight = function (...providedArgs) {
        const placeholderLength = partialArgs.filter(arg => arg === placeholder).length;
        const rangeLength = Math.max(providedArgs.length - placeholderLength, 0);
        const remainingArgs = providedArgs.slice(0, rangeLength);
        let providedArgsIndex = rangeLength;
        const substitutedArgs = partialArgs
            .slice()
            .map(arg => (arg === placeholder ? providedArgs[providedArgsIndex++] : arg));
        return func.apply(this, remainingArgs.concat(substitutedArgs));
    };
    if (func.prototype) {
        partialedRight.prototype = Object.create(func.prototype);
    }
    return partialedRight;
}
const placeholderSymbol = Symbol('partialRight.placeholder');
partialRight.placeholder = placeholderSymbol;

exports.partialRight = partialRight;
exports.partialRightImpl = partialRightImpl;
