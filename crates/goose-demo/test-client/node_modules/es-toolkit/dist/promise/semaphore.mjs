class Semaphore {
    capacity;
    available;
    deferredTasks = [];
    constructor(capacity) {
        this.capacity = capacity;
        this.available = capacity;
    }
    async acquire() {
        if (this.available > 0) {
            this.available--;
            return;
        }
        return new Promise(resolve => {
            this.deferredTasks.push(resolve);
        });
    }
    release() {
        const deferredTask = this.deferredTasks.shift();
        if (deferredTask != null) {
            deferredTask();
            return;
        }
        if (this.available < this.capacity) {
            this.available++;
        }
    }
}

export { Semaphore };
