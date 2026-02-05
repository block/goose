import { Buffer } from 'node:buffer';
export declare const nonAlphanumericKeys: string[];
type ParsedKey = {
    name: string;
    ctrl: boolean;
    meta: boolean;
    shift: boolean;
    option: boolean;
    sequence: string;
    raw: string | undefined;
    code?: string;
};
declare const parseKeypress: (s?: Buffer | string) => ParsedKey;
export default parseKeypress;
