import { rest as rest$1 } from '../../function/rest.mjs';

function rest(func, start = func.length - 1) {
    start = Number.parseInt(start, 10);
    if (Number.isNaN(start) || start < 0) {
        start = func.length - 1;
    }
    return rest$1(func, start);
}

export { rest };
