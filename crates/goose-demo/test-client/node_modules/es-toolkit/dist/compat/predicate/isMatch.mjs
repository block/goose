import { isMatchWith } from './isMatchWith.mjs';

function isMatch(target, source) {
    return isMatchWith(target, source, () => undefined);
}

export { isMatch };
