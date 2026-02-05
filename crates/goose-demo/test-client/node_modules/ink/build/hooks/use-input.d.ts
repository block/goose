/**
 * Handy information about a key that was pressed.
 */
export type Key = {
    /**
     * Up arrow key was pressed.
     */
    upArrow: boolean;
    /**
     * Down arrow key was pressed.
     */
    downArrow: boolean;
    /**
     * Left arrow key was pressed.
     */
    leftArrow: boolean;
    /**
     * Right arrow key was pressed.
     */
    rightArrow: boolean;
    /**
     * Page Down key was pressed.
     */
    pageDown: boolean;
    /**
     * Page Up key was pressed.
     */
    pageUp: boolean;
    /**
     * Return (Enter) key was pressed.
     */
    return: boolean;
    /**
     * Escape key was pressed.
     */
    escape: boolean;
    /**
     * Ctrl key was pressed.
     */
    ctrl: boolean;
    /**
     * Shift key was pressed.
     */
    shift: boolean;
    /**
     * Tab key was pressed.
     */
    tab: boolean;
    /**
     * Backspace key was pressed.
     */
    backspace: boolean;
    /**
     * Delete key was pressed.
     */
    delete: boolean;
    /**
     * [Meta key](https://en.wikipedia.org/wiki/Meta_key) was pressed.
     */
    meta: boolean;
};
type Handler = (input: string, key: Key) => void;
type Options = {
    /**
     * Enable or disable capturing of user input.
     * Useful when there are multiple useInput hooks used at once to avoid handling the same input several times.
     *
     * @default true
     */
    isActive?: boolean;
};
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
declare const useInput: (inputHandler: Handler, options?: Options) => void;
export default useInput;
