function debounce(func, debounceMs, { signal, edges } = {}) {
    let pendingThis = undefined;
    let pendingArgs = null;
    const leading = edges != null && edges.includes('leading');
    const trailing = edges == null || edges.includes('trailing');
    const invoke = () => {
        if (pendingArgs !== null) {
            func.apply(pendingThis, pendingArgs);
            pendingThis = undefined;
            pendingArgs = null;
        }
    };
    const onTimerEnd = () => {
        if (trailing) {
            invoke();
        }
        cancel();
    };
    let timeoutId = null;
    const schedule = () => {
        if (timeoutId != null) {
            clearTimeout(timeoutId);
        }
        timeoutId = setTimeout(() => {
            timeoutId = null;
            onTimerEnd();
        }, debounceMs);
    };
    const cancelTimer = () => {
        if (timeoutId !== null) {
            clearTimeout(timeoutId);
            timeoutId = null;
        }
    };
    const cancel = () => {
        cancelTimer();
        pendingThis = undefined;
        pendingArgs = null;
    };
    const flush = () => {
        invoke();
    };
    const debounced = function (...args) {
        if (signal?.aborted) {
            return;
        }
        pendingThis = this;
        pendingArgs = args;
        const isFirstCall = timeoutId == null;
        schedule();
        if (leading && isFirstCall) {
            invoke();
        }
    };
    debounced.schedule = schedule;
    debounced.cancel = cancel;
    debounced.flush = flush;
    signal?.addEventListener('abort', cancel, { once: true });
    return debounced;
}

export { debounce };
