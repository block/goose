import { type DOMElement } from './dom.js';
import type Output from './output.js';
export type OutputTransformer = (s: string, index: number) => string;
declare const renderNodeToOutput: (node: DOMElement, output: Output, options: {
    offsetX?: number;
    offsetY?: number;
    transformers?: OutputTransformer[];
    skipStaticElements: boolean;
}) => void;
export default renderNodeToOutput;
