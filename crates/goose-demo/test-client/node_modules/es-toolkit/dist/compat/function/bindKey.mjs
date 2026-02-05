function bindKey(object, key, ...partialArgs) {
    const bound = function (...providedArgs) {
        const args = [];
        let startIndex = 0;
        for (let i = 0; i < partialArgs.length; i++) {
            const arg = partialArgs[i];
            if (arg === bindKey.placeholder) {
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
            return new object[key](...args);
        }
        return object[key].apply(object, args);
    };
    return bound;
}
const bindKeyPlaceholder = Symbol('bindKey.placeholder');
bindKey.placeholder = bindKeyPlaceholder;

export { bindKey };
