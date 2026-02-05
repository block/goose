import { words } from './words.mjs';

function upperCase(str) {
    const words$1 = words(str);
    let result = '';
    for (let i = 0; i < words$1.length; i++) {
        result += words$1[i].toUpperCase();
        if (i < words$1.length - 1) {
            result += ' ';
        }
    }
    return result;
}

export { upperCase };
