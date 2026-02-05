import { debounce } from './debounce.mjs';

function throttle(func, throttleMs, { signal, edges = ['leading', 'trailing'] } = {}) {
    let pendingAt = null;
    const debounced = debounce(function (...args) {
        pendingAt = Date.now();
        func.apply(this, args);
    }, throttleMs, { signal, edges });
    const throttled = function (...args) {
        if (pendingAt == null) {
            pendingAt = Date.now();
        }
        if (Date.now() - pendingAt >= throttleMs) {
            pendingAt = Date.now();
            func.apply(this, args);
            debounced.cancel();
            debounced.schedule();
            return;
        }
        debounced.apply(this, args);
    };
    throttled.cancel = debounced.cancel;
    throttled.flush = debounced.flush;
    return throttled;
}

export { throttle };
