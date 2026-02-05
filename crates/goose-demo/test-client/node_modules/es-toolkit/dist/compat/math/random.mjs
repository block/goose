import { clamp } from './clamp.mjs';
import { random as random$1 } from '../../math/random.mjs';
import { randomInt } from '../../math/randomInt.mjs';

function random(...args) {
    let minimum = 0;
    let maximum = 1;
    let floating = false;
    switch (args.length) {
        case 1: {
            if (typeof args[0] === 'boolean') {
                floating = args[0];
            }
            else {
                maximum = args[0];
            }
            break;
        }
        case 2: {
            if (typeof args[1] === 'boolean') {
                maximum = args[0];
                floating = args[1];
            }
            else {
                minimum = args[0];
                maximum = args[1];
            }
        }
        case 3: {
            if (typeof args[2] === 'object' && args[2] != null && args[2][args[1]] === args[0]) {
                minimum = 0;
                maximum = args[0];
                floating = false;
            }
            else {
                minimum = args[0];
                maximum = args[1];
                floating = args[2];
            }
        }
    }
    if (typeof minimum !== 'number') {
        minimum = Number(minimum);
    }
    if (typeof maximum !== 'number') {
        minimum = Number(maximum);
    }
    if (!minimum) {
        minimum = 0;
    }
    if (!maximum) {
        maximum = 0;
    }
    if (minimum > maximum) {
        [minimum, maximum] = [maximum, minimum];
    }
    minimum = clamp(minimum, -Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
    maximum = clamp(maximum, -Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
    if (minimum === maximum) {
        return minimum;
    }
    if (floating) {
        return random$1(minimum, maximum + 1);
    }
    else {
        return randomInt(minimum, maximum + 1);
    }
}

export { random };
