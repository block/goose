import { conformsTo } from './conformsTo.mjs';
import { cloneDeep } from '../../object/cloneDeep.mjs';

function conforms(source) {
    source = cloneDeep(source);
    return function (object) {
        return conformsTo(object, source);
    };
}

export { conforms };
