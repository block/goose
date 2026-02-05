import { ary } from './ary.mjs';

function unary(func) {
    return ary(func, 1);
}

export { unary };
