import { sumBy } from './sumBy.mjs';

function meanBy(items, getValue) {
    return sumBy(items, item => getValue(item)) / items.length;
}

export { meanBy };
