'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const semaphore = require('./semaphore.js');

class Mutex {
    semaphore = new semaphore.Semaphore(1);
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

exports.Mutex = Mutex;
