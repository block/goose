import { isBlob } from './isBlob.mjs';

function isFile(x) {
    if (typeof File === 'undefined') {
        return false;
    }
    return isBlob(x) && x instanceof File;
}

export { isFile };
