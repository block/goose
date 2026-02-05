import { Semaphore } from '../promise/semaphore.mjs';

function limitAsync(callback, concurrency) {
    const semaphore = new Semaphore(concurrency);
    return async function (...args) {
        try {
            await semaphore.acquire();
            return await callback.apply(this, args);
        }
        finally {
            semaphore.release();
        }
    };
}

export { limitAsync };
