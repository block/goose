import { flatten } from '../array/flatten.mjs';

function rearg(func, ...indices) {
    const flattenIndices = flatten(indices);
    return function (...args) {
        const reorderedArgs = flattenIndices.map(i => args[i]).slice(0, args.length);
        for (let i = reorderedArgs.length; i < args.length; i++) {
            reorderedArgs.push(args[i]);
        }
        return func.apply(this, reorderedArgs);
    };
}

export { rearg };
