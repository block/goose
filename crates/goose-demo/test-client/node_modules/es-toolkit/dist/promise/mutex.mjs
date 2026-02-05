import { Semaphore } from './semaphore.mjs';

class Mutex {
    semaphore = new Semaphore(1);
    get isLocked() {
        return this.semaphore.available === 0;
    }
    async acquire() {
        return this.semaphore.acquire();
    }
    release() {
        this.semaphore.release();
    }
}

export { Mutex };
