import { iteratee } from './iteratee.mjs';

function over(...iteratees) {
    if (iteratees.length === 1 && Array.isArray(iteratees[0])) {
        iteratees = iteratees[0];
    }
    const funcs = iteratees.map(item => iteratee(item));
    return function (...args) {
        return funcs.map(func => func.apply(this, args));
    };
}

export { over };
