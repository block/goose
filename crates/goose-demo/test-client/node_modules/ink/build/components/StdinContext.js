import { EventEmitter } from 'node:events';
import process from 'node:process';
import { createContext } from 'react';
/**
 * `StdinContext` is a React context, which exposes input stream.
 */
// eslint-disable-next-line @typescript-eslint/naming-convention
const StdinContext = createContext({
    stdin: process.stdin,
    // eslint-disable-next-line @typescript-eslint/naming-convention
    internal_eventEmitter: new EventEmitter(),
    setRawMode() { },
    isRawModeSupported: false,
    // eslint-disable-next-line @typescript-eslint/naming-convention
    internal_exitOnCtrlC: true,
});
StdinContext.displayName = 'InternalStdinContext';
export default StdinContext;
//# sourceMappingURL=StdinContext.js.map