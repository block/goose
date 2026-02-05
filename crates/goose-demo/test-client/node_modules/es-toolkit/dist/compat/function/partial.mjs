import { partialImpl } from '../../function/partial.mjs';

function partial(func, ...partialArgs) {
    return partialImpl(func, partial.placeholder, ...partialArgs);
}
partial.placeholder = Symbol('compat.partial.placeholder');

export { partial };
