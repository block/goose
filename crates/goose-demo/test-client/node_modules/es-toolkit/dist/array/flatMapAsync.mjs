import { flatten } from './flatten.mjs';
import { limitAsync } from './limitAsync.mjs';

async function flatMapAsync(array, callback, options) {
    if (options?.concurrency != null) {
        callback = limitAsync(callback, options.concurrency);
    }
    const results = await Promise.all(array.map(callback));
    return flatten(results);
}

export { flatMapAsync };
