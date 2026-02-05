import { toInteger } from '../util/toInteger.mjs';

function nthArg(n = 0) {
    return function (...args) {
        return args.at(toInteger(n));
    };
}

export { nthArg };
