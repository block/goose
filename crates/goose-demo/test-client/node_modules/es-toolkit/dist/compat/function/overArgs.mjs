import { identity } from '../../function/identity.mjs';
import { iteratee } from '../util/iteratee.mjs';

function overArgs(func, ..._transforms) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    const transforms = _transforms.flat();
    return function (...args) {
        const length = Math.min(args.length, transforms.length);
        const transformedArgs = [...args];
        for (let i = 0; i < length; i++) {
            const transform = iteratee(transforms[i] ?? identity);
            transformedArgs[i] = transform.call(this, args[i]);
        }
        return func.apply(this, transformedArgs);
    };
}

export { overArgs };
