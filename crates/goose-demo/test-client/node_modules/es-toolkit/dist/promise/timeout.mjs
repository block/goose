import { delay } from './delay.mjs';
import { TimeoutError } from '../error/TimeoutError.mjs';

async function timeout(ms) {
    await delay(ms);
    throw new TimeoutError();
}

export { timeout };
