'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function bind(func, thisObj, ...partialArgs) {
    const bound = function (...providedArgs) {
        const args = [];
        let startIndex = 0;
        for (let i = 0; i < partialArgs.length; i++) {
            const arg = partialArgs[i];
            if (arg === bind.placeholder) {
                args.push(providedArgs[startIndex++]);
            }
            else {
                args.push(arg);
            }
        }
        for (let i = startIndex; i < providedArgs.length; i++) {
            args.push(providedArgs[i]);
        }
        if (this instanceof bound) {
            return new func(...args);
        }
        return func.apply(thisObj, args);
    };
    return bound;
}
const bindPlaceholder = Symbol('bind.placeholder');
bind.placeholder = bindPlaceholder;

exports.bind = bind;
