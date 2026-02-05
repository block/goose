import { limitAsync } from './limitAsync.mjs';

function mapAsync(array, callback, options) {
    if (options?.concurrency != null) {
        callback = limitAsync(callback, options.concurrency);
    }
    return Promise.all(array.map(callback));
}

export { mapAsync };
