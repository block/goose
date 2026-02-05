import { type DOMElement } from './dom.js';
type Output = {
    /**
     * Element width.
     */
    width: number;
    /**
     * Element height.
     */
    height: number;
};
/**
 * Measure the dimensions of a particular `<Box>` element.
 */
declare const measureElement: (node: DOMElement) => Output;
export default measureElement;
