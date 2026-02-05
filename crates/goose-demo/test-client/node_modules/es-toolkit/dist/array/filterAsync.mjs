import { limitAsync } from './limitAsync.mjs';

async function filterAsync(array, predicate, options) {
    if (options?.concurrency != null) {
        predicate = limitAsync(predicate, options.concurrency);
    }
    const results = await Promise.all(array.map(predicate));
    return array.filter((_, index) => results[index]);
}

export { filterAsync };
