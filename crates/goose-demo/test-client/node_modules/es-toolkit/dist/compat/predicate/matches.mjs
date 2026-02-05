import { isMatch } from './isMatch.mjs';
import { cloneDeep } from '../../object/cloneDeep.mjs';

function matches(source) {
    source = cloneDeep(source);
    return (target) => {
        return isMatch(target, source);
    };
}

export { matches };
