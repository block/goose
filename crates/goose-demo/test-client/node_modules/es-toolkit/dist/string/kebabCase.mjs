import { words } from './words.mjs';

function kebabCase(str) {
    const words$1 = words(str);
    return words$1.map(word => word.toLowerCase()).join('-');
}

export { kebabCase };
