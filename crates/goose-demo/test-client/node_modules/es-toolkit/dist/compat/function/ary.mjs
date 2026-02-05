import { ary as ary$1 } from '../../function/ary.mjs';

function ary(func, n = func.length, guard) {
    if (guard) {
        n = func.length;
    }
    if (Number.isNaN(n) || n < 0) {
        n = 0;
    }
    return ary$1(func, n);
}

export { ary };
