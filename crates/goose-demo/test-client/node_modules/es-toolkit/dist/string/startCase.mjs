import { words } from './words.mjs';

function startCase(str) {
    const words$1 = words(str.trim());
    let result = '';
    for (let i = 0; i < words$1.length; i++) {
        const word = words$1[i];
        if (result) {
            result += ' ';
        }
        result += word[0].toUpperCase() + word.slice(1).toLowerCase();
    }
    return result;
}

export { startCase };
