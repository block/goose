import Yoga from 'yoga-layout';
import measureText from './measure-text.js';
import wrapText from './wrap-text.js';
import squashTextNodes from './squash-text-nodes.js';
export const createNode = (nodeName) => {
    const node = {
        nodeName,
        style: {},
        attributes: {},
        childNodes: [],
        parentNode: undefined,
        yogaNode: nodeName === 'ink-virtual-text' ? undefined : Yoga.Node.create(),
    };
    if (nodeName === 'ink-text') {
        node.yogaNode?.setMeasureFunc(measureTextNode.bind(null, node));
    }
    return node;
};
export const appendChildNode = (node, childNode) => {
    if (childNode.parentNode) {
        removeChildNode(childNode.parentNode, childNode);
    }
    childNode.parentNode = node;
    node.childNodes.push(childNode);
    if (childNode.yogaNode) {
        node.yogaNode?.insertChild(childNode.yogaNode, node.yogaNode.getChildCount());
    }
    if (node.nodeName === 'ink-text' || node.nodeName === 'ink-virtual-text') {
        markNodeAsDirty(node);
    }
};
export const insertBeforeNode = (node, newChildNode, beforeChildNode) => {
    if (newChildNode.parentNode) {
        removeChildNode(newChildNode.parentNode, newChildNode);
    }
    newChildNode.parentNode = node;
    const index = node.childNodes.indexOf(beforeChildNode);
    if (index >= 0) {
        node.childNodes.splice(index, 0, newChildNode);
        if (newChildNode.yogaNode) {
            node.yogaNode?.insertChild(newChildNode.yogaNode, index);
        }
        return;
    }
    node.childNodes.push(newChildNode);
    if (newChildNode.yogaNode) {
        node.yogaNode?.insertChild(newChildNode.yogaNode, node.yogaNode.getChildCount());
    }
    if (node.nodeName === 'ink-text' || node.nodeName === 'ink-virtual-text') {
        markNodeAsDirty(node);
    }
};
export const removeChildNode = (node, removeNode) => {
    if (removeNode.yogaNode) {
        removeNode.parentNode?.yogaNode?.removeChild(removeNode.yogaNode);
    }
    removeNode.parentNode = undefined;
    const index = node.childNodes.indexOf(removeNode);
    if (index >= 0) {
        node.childNodes.splice(index, 1);
    }
    if (node.nodeName === 'ink-text' || node.nodeName === 'ink-virtual-text') {
        markNodeAsDirty(node);
    }
};
export const setAttribute = (node, key, value) => {
    node.attributes[key] = value;
};
export const setStyle = (node, style) => {
    node.style = style;
};
export const createTextNode = (text) => {
    const node = {
        nodeName: '#text',
        nodeValue: text,
        yogaNode: undefined,
        parentNode: undefined,
        style: {},
    };
    setTextNodeValue(node, text);
    return node;
};
const measureTextNode = function (node, width) {
    const text = node.nodeName === '#text' ? node.nodeValue : squashTextNodes(node);
    const dimensions = measureText(text);
    // Text fits into container, no need to wrap
    if (dimensions.width <= width) {
        return dimensions;
    }
    // This is happening when <Box> is shrinking child nodes and Yoga asks
    // if we can fit this text node in a <1px space, so we just tell Yoga "no"
    if (dimensions.width >= 1 && width > 0 && width < 1) {
        return dimensions;
    }
    const textWrap = node.style?.textWrap ?? 'wrap';
    const wrappedText = wrapText(text, width, textWrap);
    return measureText(wrappedText);
};
const findClosestYogaNode = (node) => {
    if (!node?.parentNode) {
        return undefined;
    }
    return node.yogaNode ?? findClosestYogaNode(node.parentNode);
};
const markNodeAsDirty = (node) => {
    // Mark closest Yoga node as dirty to measure text dimensions again
    const yogaNode = findClosestYogaNode(node);
    yogaNode?.markDirty();
};
export const setTextNodeValue = (node, text) => {
    if (typeof text !== 'string') {
        text = String(text);
    }
    node.nodeValue = text;
    markNodeAsDirty(node);
};
//# sourceMappingURL=dom.js.map