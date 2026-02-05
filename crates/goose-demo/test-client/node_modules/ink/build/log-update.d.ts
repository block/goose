import { type Writable } from 'node:stream';
export type LogUpdate = {
    clear: () => void;
    done: () => void;
    (str: string): void;
};
declare const logUpdate: {
    create: (stream: Writable, { showCursor }?: {
        showCursor?: boolean | undefined;
    }) => LogUpdate;
};
export default logUpdate;
