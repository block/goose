import { partialRightImpl } from '../../function/partialRight.mjs';

function partialRight(func, ...partialArgs) {
    return partialRightImpl(func, partialRight.placeholder, ...partialArgs);
}
partialRight.placeholder = Symbol('compat.partialRight.placeholder');

export { partialRight };
