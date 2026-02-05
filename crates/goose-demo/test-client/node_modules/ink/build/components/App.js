import { EventEmitter } from 'node:events';
import process from 'node:process';
import React, { PureComponent } from 'react';
import cliCursor from 'cli-cursor';
import AppContext from './AppContext.js';
import StdinContext from './StdinContext.js';
import StdoutContext from './StdoutContext.js';
import StderrContext from './StderrContext.js';
import FocusContext from './FocusContext.js';
import ErrorOverview from './ErrorOverview.js';
const tab = '\t';
const shiftTab = '\u001B[Z';
const escape = '\u001B';
// Root component for all Ink apps
// It renders stdin and stdout contexts, so that children can access them if needed
// It also handles Ctrl+C exiting and cursor visibility
export default class App extends PureComponent {
    static displayName = 'InternalApp';
    static getDerivedStateFromError(error) {
        return { error };
    }
    state = {
        isFocusEnabled: true,
        activeFocusId: undefined,
        focusables: [],
        error: undefined,
    };
    // Count how many components enabled raw mode to avoid disabling
    // raw mode until all components don't need it anymore
    rawModeEnabledCount = 0;
    // eslint-disable-next-line @typescript-eslint/naming-convention
    internal_eventEmitter = new EventEmitter();
    // Determines if TTY is supported on the provided stdin
    isRawModeSupported() {
        return this.props.stdin.isTTY;
    }
    render() {
        return (React.createElement(AppContext.Provider
        // eslint-disable-next-line react/jsx-no-constructed-context-values
        , { 
            // eslint-disable-next-line react/jsx-no-constructed-context-values
            value: {
                exit: this.handleExit,
            } },
            React.createElement(StdinContext.Provider
            // eslint-disable-next-line react/jsx-no-constructed-context-values
            , { 
                // eslint-disable-next-line react/jsx-no-constructed-context-values
                value: {
                    stdin: this.props.stdin,
                    setRawMode: this.handleSetRawMode,
                    isRawModeSupported: this.isRawModeSupported(),
                    // eslint-disable-next-line @typescript-eslint/naming-convention
                    internal_exitOnCtrlC: this.props.exitOnCtrlC,
                    // eslint-disable-next-line @typescript-eslint/naming-convention
                    internal_eventEmitter: this.internal_eventEmitter,
                } },
                React.createElement(StdoutContext.Provider
                // eslint-disable-next-line react/jsx-no-constructed-context-values
                , { 
                    // eslint-disable-next-line react/jsx-no-constructed-context-values
                    value: {
                        stdout: this.props.stdout,
                        write: this.props.writeToStdout,
                    } },
                    React.createElement(StderrContext.Provider
                    // eslint-disable-next-line react/jsx-no-constructed-context-values
                    , { 
                        // eslint-disable-next-line react/jsx-no-constructed-context-values
                        value: {
                            stderr: this.props.stderr,
                            write: this.props.writeToStderr,
                        } },
                        React.createElement(FocusContext.Provider
                        // eslint-disable-next-line react/jsx-no-constructed-context-values
                        , { 
                            // eslint-disable-next-line react/jsx-no-constructed-context-values
                            value: {
                                activeId: this.state.activeFocusId,
                                add: this.addFocusable,
                                remove: this.removeFocusable,
                                activate: this.activateFocusable,
                                deactivate: this.deactivateFocusable,
                                enableFocus: this.enableFocus,
                                disableFocus: this.disableFocus,
                                focusNext: this.focusNext,
                                focusPrevious: this.focusPrevious,
                                focus: this.focus,
                            } }, this.state.error ? (React.createElement(ErrorOverview, { error: this.state.error })) : (this.props.children)))))));
    }
    componentDidMount() {
        cliCursor.hide(this.props.stdout);
    }
    componentWillUnmount() {
        cliCursor.show(this.props.stdout);
        // ignore calling setRawMode on an handle stdin it cannot be called
        if (this.isRawModeSupported()) {
            this.handleSetRawMode(false);
        }
    }
    componentDidCatch(error) {
        this.handleExit(error);
    }
    handleSetRawMode = (isEnabled) => {
        const { stdin } = this.props;
        if (!this.isRawModeSupported()) {
            if (stdin === process.stdin) {
                throw new Error('Raw mode is not supported on the current process.stdin, which Ink uses as input stream by default.\nRead about how to prevent this error on https://github.com/vadimdemedes/ink/#israwmodesupported');
            }
            else {
                throw new Error('Raw mode is not supported on the stdin provided to Ink.\nRead about how to prevent this error on https://github.com/vadimdemedes/ink/#israwmodesupported');
            }
        }
        stdin.setEncoding('utf8');
        if (isEnabled) {
            // Ensure raw mode is enabled only once
            if (this.rawModeEnabledCount === 0) {
                stdin.ref();
                stdin.setRawMode(true);
                stdin.addListener('readable', this.handleReadable);
            }
            this.rawModeEnabledCount++;
            return;
        }
        // Disable raw mode only when no components left that are using it
        if (--this.rawModeEnabledCount === 0) {
            stdin.setRawMode(false);
            stdin.removeListener('readable', this.handleReadable);
            stdin.unref();
        }
    };
    handleReadable = () => {
        let chunk;
        // eslint-disable-next-line @typescript-eslint/ban-types
        while ((chunk = this.props.stdin.read()) !== null) {
            this.handleInput(chunk);
            this.internal_eventEmitter.emit('input', chunk);
        }
    };
    handleInput = (input) => {
        // Exit on Ctrl+C
        // eslint-disable-next-line unicorn/no-hex-escape
        if (input === '\x03' && this.props.exitOnCtrlC) {
            this.handleExit();
        }
        // Reset focus when there's an active focused component on Esc
        if (input === escape && this.state.activeFocusId) {
            this.setState({
                activeFocusId: undefined,
            });
        }
        if (this.state.isFocusEnabled && this.state.focusables.length > 0) {
            if (input === tab) {
                this.focusNext();
            }
            if (input === shiftTab) {
                this.focusPrevious();
            }
        }
    };
    handleExit = (error) => {
        if (this.isRawModeSupported()) {
            this.handleSetRawMode(false);
        }
        this.props.onExit(error);
    };
    enableFocus = () => {
        this.setState({
            isFocusEnabled: true,
        });
    };
    disableFocus = () => {
        this.setState({
            isFocusEnabled: false,
        });
    };
    focus = (id) => {
        this.setState(previousState => {
            const hasFocusableId = previousState.focusables.some(focusable => focusable?.id === id);
            if (!hasFocusableId) {
                return previousState;
            }
            return { activeFocusId: id };
        });
    };
    focusNext = () => {
        this.setState(previousState => {
            const firstFocusableId = previousState.focusables.find(focusable => focusable.isActive)?.id;
            const nextFocusableId = this.findNextFocusable(previousState);
            return {
                activeFocusId: nextFocusableId ?? firstFocusableId,
            };
        });
    };
    focusPrevious = () => {
        this.setState(previousState => {
            const lastFocusableId = previousState.focusables.findLast(focusable => focusable.isActive)?.id;
            const previousFocusableId = this.findPreviousFocusable(previousState);
            return {
                activeFocusId: previousFocusableId ?? lastFocusableId,
            };
        });
    };
    addFocusable = (id, { autoFocus }) => {
        this.setState(previousState => {
            let nextFocusId = previousState.activeFocusId;
            if (!nextFocusId && autoFocus) {
                nextFocusId = id;
            }
            return {
                activeFocusId: nextFocusId,
                focusables: [
                    ...previousState.focusables,
                    {
                        id,
                        isActive: true,
                    },
                ],
            };
        });
    };
    removeFocusable = (id) => {
        this.setState(previousState => ({
            activeFocusId: previousState.activeFocusId === id
                ? undefined
                : previousState.activeFocusId,
            focusables: previousState.focusables.filter(focusable => {
                return focusable.id !== id;
            }),
        }));
    };
    activateFocusable = (id) => {
        this.setState(previousState => ({
            focusables: previousState.focusables.map(focusable => {
                if (focusable.id !== id) {
                    return focusable;
                }
                return {
                    id,
                    isActive: true,
                };
            }),
        }));
    };
    deactivateFocusable = (id) => {
        this.setState(previousState => ({
            activeFocusId: previousState.activeFocusId === id
                ? undefined
                : previousState.activeFocusId,
            focusables: previousState.focusables.map(focusable => {
                if (focusable.id !== id) {
                    return focusable;
                }
                return {
                    id,
                    isActive: false,
                };
            }),
        }));
    };
    findNextFocusable = (state) => {
        const activeIndex = state.focusables.findIndex(focusable => {
            return focusable.id === state.activeFocusId;
        });
        for (let index = activeIndex + 1; index < state.focusables.length; index++) {
            const focusable = state.focusables[index];
            if (focusable?.isActive) {
                return focusable.id;
            }
        }
        return undefined;
    };
    findPreviousFocusable = (state) => {
        const activeIndex = state.focusables.findIndex(focusable => {
            return focusable.id === state.activeFocusId;
        });
        for (let index = activeIndex - 1; index >= 0; index--) {
            const focusable = state.focusables[index];
            if (focusable?.isActive) {
                return focusable.id;
            }
        }
        return undefined;
    };
}
//# sourceMappingURL=App.js.map