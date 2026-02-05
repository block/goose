import { limitAsync } from './limitAsync.mjs';

async function forEachAsync(array, callback, options) {
    if (options?.concurrency != null) {
        callback = limitAsync(callback, options.concurrency);
    }
    await Promise.all(array.map(callback));
}

export { forEachAsync };
