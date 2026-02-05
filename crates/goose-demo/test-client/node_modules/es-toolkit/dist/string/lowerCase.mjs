import { words } from './words.mjs';

function lowerCase(str) {
    const words$1 = words(str);
    return words$1.map(word => word.toLowerCase()).join(' ');
}

export { lowerCase };
