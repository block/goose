function curry(func, arity = func.length, guard) {
    arity = guard ? func.length : arity;
    arity = Number.parseInt(arity, 10);
    if (Number.isNaN(arity) || arity < 1) {
        arity = 0;
    }
    const wrapper = function (...partialArgs) {
        const holders = partialArgs.filter(item => item === curry.placeholder);
        const length = partialArgs.length - holders.length;
        if (length < arity) {
            return makeCurry(func, arity - length, partialArgs);
        }
        if (this instanceof wrapper) {
            return new func(...partialArgs);
        }
        return func.apply(this, partialArgs);
    };
    wrapper.placeholder = curryPlaceholder;
    return wrapper;
}
function makeCurry(func, arity, partialArgs) {
    function wrapper(...providedArgs) {
        const holders = providedArgs.filter(item => item === curry.placeholder);
        const length = providedArgs.length - holders.length;
        providedArgs = composeArgs(providedArgs, partialArgs);
        if (length < arity) {
            return makeCurry(func, arity - length, providedArgs);
        }
        if (this instanceof wrapper) {
            return new func(...providedArgs);
        }
        return func.apply(this, providedArgs);
    }
    wrapper.placeholder = curryPlaceholder;
    return wrapper;
}
function composeArgs(providedArgs, partialArgs) {
    const args = [];
    let startIndex = 0;
    for (let i = 0; i < partialArgs.length; i++) {
        const arg = partialArgs[i];
        if (arg === curry.placeholder && startIndex < providedArgs.length) {
            args.push(providedArgs[startIndex++]);
        }
        else {
            args.push(arg);
        }
    }
    for (let i = startIndex; i < providedArgs.length; i++) {
        args.push(providedArgs[i]);
    }
    return args;
}
const curryPlaceholder = Symbol('curry.placeholder');
curry.placeholder = curryPlaceholder;

export { curry };
