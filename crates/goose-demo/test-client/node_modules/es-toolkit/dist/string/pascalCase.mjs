import { capitalize } from './capitalize.mjs';
import { words } from './words.mjs';

function pascalCase(str) {
    const words$1 = words(str);
    return words$1.map(word => capitalize(word)).join('');
}

export { pascalCase };
