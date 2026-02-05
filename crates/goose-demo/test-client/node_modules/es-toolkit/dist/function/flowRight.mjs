import { flow } from './flow.mjs';

function flowRight(...funcs) {
    return flow(...funcs.reverse());
}

export { flowRight };
