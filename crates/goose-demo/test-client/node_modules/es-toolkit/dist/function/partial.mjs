function partial(func, ...partialArgs) {
    return partialImpl(func, placeholderSymbol, ...partialArgs);
}
function partialImpl(func, placeholder, ...partialArgs) {
    const partialed = function (...providedArgs) {
        let providedArgsIndex = 0;
        const substitutedArgs = partialArgs
            .slice()
            .map(arg => (arg === placeholder ? providedArgs[providedArgsIndex++] : arg));
        const remainingArgs = providedArgs.slice(providedArgsIndex);
        return func.apply(this, substitutedArgs.concat(remainingArgs));
    };
    if (func.prototype) {
        partialed.prototype = Object.create(func.prototype);
    }
    return partialed;
}
const placeholderSymbol = Symbol('partial.placeholder');
partial.placeholder = placeholderSymbol;

export { partial, partialImpl };
