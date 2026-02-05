function curryRight(func, arity = func.length, guard) {
    arity = guard ? func.length : arity;
    arity = Number.parseInt(arity, 10);
    if (Number.isNaN(arity) || arity < 1) {
        arity = 0;
    }
    const wrapper = function (...partialArgs) {
        const holders = partialArgs.filter(item => item === curryRight.placeholder);
        const length = partialArgs.length - holders.length;
        if (length < arity) {
            return makeCurryRight(func, arity - length, partialArgs);
        }
        if (this instanceof wrapper) {
            return new func(...partialArgs);
        }
        return func.apply(this, partialArgs);
    };
    wrapper.placeholder = curryRightPlaceholder;
    return wrapper;
}
function makeCurryRight(func, arity, partialArgs) {
    function wrapper(...providedArgs) {
        const holders = providedArgs.filter(item => item === curryRight.placeholder);
        const length = providedArgs.length - holders.length;
        providedArgs = composeArgs(providedArgs, partialArgs);
        if (length < arity) {
            return makeCurryRight(func, arity - length, providedArgs);
        }
        if (this instanceof wrapper) {
            return new func(...providedArgs);
        }
        return func.apply(this, providedArgs);
    }
    wrapper.placeholder = curryRightPlaceholder;
    return wrapper;
}
function composeArgs(providedArgs, partialArgs) {
    const placeholderLength = partialArgs.filter(arg => arg === curryRight.placeholder).length;
    const rangeLength = Math.max(providedArgs.length - placeholderLength, 0);
    const args = [];
    let providedIndex = 0;
    for (let i = 0; i < rangeLength; i++) {
        args.push(providedArgs[providedIndex++]);
    }
    for (let i = 0; i < partialArgs.length; i++) {
        const arg = partialArgs[i];
        if (arg === curryRight.placeholder) {
            if (providedIndex < providedArgs.length) {
                args.push(providedArgs[providedIndex++]);
            }
            else {
                args.push(arg);
            }
        }
        else {
            args.push(arg);
        }
    }
    return args;
}
const curryRightPlaceholder = Symbol('curryRight.placeholder');
curryRight.placeholder = curryRightPlaceholder;

export { curryRight };
