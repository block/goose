import { Stream } from 'node:stream';
import process from 'node:process';
import Ink from './ink.js';
import instances from './instances.js';
/**
 * Mount a component and render the output.
 */
const render = (node, options) => {
    const inkOptions = {
        stdout: process.stdout,
        stdin: process.stdin,
        stderr: process.stderr,
        debug: false,
        exitOnCtrlC: true,
        patchConsole: true,
        ...getOptions(options),
    };
    const instance = getInstance(inkOptions.stdout, () => new Ink(inkOptions));
    instance.render(node);
    return {
        rerender: instance.render,
        unmount() {
            instance.unmount();
        },
        waitUntilExit: instance.waitUntilExit,
        cleanup: () => instances.delete(inkOptions.stdout),
        clear: instance.clear,
    };
};
export default render;
const getOptions = (stdout = {}) => {
    if (stdout instanceof Stream) {
        return {
            stdout,
            stdin: process.stdin,
        };
    }
    return stdout;
};
const getInstance = (stdout, createInstance) => {
    let instance = instances.get(stdout);
    if (!instance) {
        instance = createInstance();
        instances.set(stdout, instance);
    }
    return instance;
};
//# sourceMappingURL=render.js.map