import { type ReactNode } from 'react';
export type Options = {
    stdout: NodeJS.WriteStream;
    stdin: NodeJS.ReadStream;
    stderr: NodeJS.WriteStream;
    debug: boolean;
    exitOnCtrlC: boolean;
    patchConsole: boolean;
    waitUntilExit?: () => Promise<void>;
};
export default class Ink {
    private readonly options;
    private readonly log;
    private readonly throttledLog;
    private isUnmounted;
    private lastOutput;
    private readonly container;
    private readonly rootNode;
    private fullStaticOutput;
    private exitPromise?;
    private restoreConsole?;
    private readonly unsubscribeResize?;
    constructor(options: Options);
    resized: () => void;
    resolveExitPromise: () => void;
    rejectExitPromise: (reason?: Error) => void;
    unsubscribeExit: () => void;
    calculateLayout: () => void;
    onRender: () => void;
    render(node: ReactNode): void;
    writeToStdout(data: string): void;
    writeToStderr(data: string): void;
    unmount(error?: Error | number | null): void;
    waitUntilExit(): Promise<void>;
    clear(): void;
    patchConsole(): void;
}
