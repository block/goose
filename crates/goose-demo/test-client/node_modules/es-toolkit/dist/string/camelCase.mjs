import { capitalize } from './capitalize.mjs';
import { words } from './words.mjs';

function camelCase(str) {
    const words$1 = words(str);
    if (words$1.length === 0) {
        return '';
    }
    const [first, ...rest] = words$1;
    return `${first.toLowerCase()}${rest.map(word => capitalize(word)).join('')}`;
}

export { camelCase };
