import { words } from './words.mjs';

function snakeCase(str) {
    const words$1 = words(str);
    return words$1.map(word => word.toLowerCase()).join('_');
}

export { snakeCase };
