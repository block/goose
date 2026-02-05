import { type DOMElement } from './dom.js';
type Result = {
    output: string;
    outputHeight: number;
    staticOutput: string;
};
declare const renderer: (node: DOMElement) => Result;
export default renderer;
