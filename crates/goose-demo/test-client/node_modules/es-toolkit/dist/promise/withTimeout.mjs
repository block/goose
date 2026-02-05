import { timeout } from './timeout.mjs';

async function withTimeout(run, ms) {
    return Promise.race([run(), timeout(ms)]);
}

export { withTimeout };
