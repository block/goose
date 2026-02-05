import { toFinite } from './toFinite.mjs';

function toInteger(value) {
    const finite = toFinite(value);
    const remainder = finite % 1;
    return remainder ? finite - remainder : finite;
}

export { toInteger };
