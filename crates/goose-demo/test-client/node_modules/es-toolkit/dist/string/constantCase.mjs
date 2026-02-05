import { words } from './words.mjs';

function constantCase(str) {
    const words$1 = words(str);
    return words$1.map(word => word.toUpperCase()).join('_');
}

export { constantCase };
