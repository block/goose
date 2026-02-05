import { useEffect } from 'react';
import parseKeypress, { nonAlphanumericKeys } from '../parse-keypress.js';
import reconciler from '../reconciler.js';
import useStdin from './use-stdin.js';
/**
 * This hook is used for handling user input.
 * It's a more convenient alternative to using `StdinContext` and listening to `data` events.
 * The callback you pass to `useInput` is called for each character when user enters any input.
 * However, if user pastes text and it's more than one character, the callback will be called only once and the whole string will be passed as `input`.
 *
 * ```
 * import {useInput} from 'ink';
 *
 * const UserInput = () => {
 *   useInput((input, key) => {
 *     if (input === 'q') {
 *       // Exit program
 *     }
 *
 *     if (key.leftArrow) {
 *       // Left arrow key pressed
 *     }
 *   });
 *
 *   return …
 * };
 * ```
 */
const useInput = (inputHandler, options = {}) => {
    // eslint-disable-next-line @typescript-eslint/naming-convention
    const { stdin, setRawMode, internal_exitOnCtrlC, internal_eventEmitter } = useStdin();
    useEffect(() => {
        if (options.isActive === false) {
            return;
        }
        setRawMode(true);
        return () => {
            setRawMode(false);
        };
    }, [options.isActive, setRawMode]);
    useEffect(() => {
        if (options.isActive === false) {
            return;
        }
        const handleData = (data) => {
            const keypress = parseKeypress(data);
            const key = {
                upArrow: keypress.name === 'up',
                downArrow: keypress.name === 'down',
                leftArrow: keypress.name === 'left',
                rightArrow: keypress.name === 'right',
                pageDown: keypress.name === 'pagedown',
                pageUp: keypress.name === 'pageup',
                return: keypress.name === 'return',
                escape: keypress.name === 'escape',
                ctrl: keypress.ctrl,
                shift: keypress.shift,
                tab: keypress.name === 'tab',
                backspace: keypress.name === 'backspace',
                delete: keypress.name === 'delete',
                // `parseKeypress` parses \u001B\u001B[A (meta + up arrow) as meta = false
                // but with option = true, so we need to take this into account here
                // to avoid breaking changes in Ink.
                // TODO(vadimdemedes): consider removing this in the next major version.
                meta: keypress.meta || keypress.name === 'escape' || keypress.option,
            };
            let input = keypress.ctrl ? keypress.name : keypress.sequence;
            if (nonAlphanumericKeys.includes(keypress.name)) {
                input = '';
            }
            // Strip meta if it's still remaining after `parseKeypress`
            // TODO(vadimdemedes): remove this in the next major version.
            if (input.startsWith('\u001B')) {
                input = input.slice(1);
            }
            if (input.length === 1 &&
                typeof input[0] === 'string' &&
                /[A-Z]/.test(input[0])) {
                key.shift = true;
            }
            // If app is not supposed to exit on Ctrl+C, then let input listener handle it
            if (!(input === 'c' && key.ctrl) || !internal_exitOnCtrlC) {
                // @ts-expect-error TypeScript types for `batchedUpdates` require an argument, but React's codebase doesn't provide it and it works without it as exepected.
                reconciler.batchedUpdates(() => {
                    inputHandler(input, key);
                });
            }
        };
        internal_eventEmitter?.on('input', handleData);
        return () => {
            internal_eventEmitter?.removeListener('input', handleData);
        };
    }, [options.isActive, stdin, internal_exitOnCtrlC, inputHandler]);
};
export default useInput;
//# sourceMappingURL=use-input.js.map