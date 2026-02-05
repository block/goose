import { random } from './random.mjs';

function randomInt(minimum, maximum) {
    return Math.floor(random(minimum, maximum));
}

export { randomInt };
