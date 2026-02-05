import { AbortError } from '../error/AbortError.mjs';

function delay(ms, { signal } = {}) {
    return new Promise((resolve, reject) => {
        const abortError = () => {
            reject(new AbortError());
        };
        const abortHandler = () => {
            clearTimeout(timeoutId);
            abortError();
        };
        if (signal?.aborted) {
            return abortError();
        }
        const timeoutId = setTimeout(() => {
            signal?.removeEventListener('abort', abortHandler);
            resolve();
        }, ms);
        signal?.addEventListener('abort', abortHandler, { once: true });
    });
}

export { delay };
