import process from 'node:process';
import createReconciler from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js';
import Yoga from 'yoga-layout';
import { createTextNode, appendChildNode, insertBeforeNode, removeChildNode, setStyle, setTextNodeValue, createNode, setAttribute, } from './dom.js';
import applyStyles from './styles.js';
// We need to conditionally perform devtools connection to avoid
// accidentally breaking other third-party code.
// See https://github.com/vadimdemedes/ink/issues/384
if (process.env['DEV'] === 'true') {
    try {
        await import('./devtools.js');
    }
    catch (error) {
        if (error.code === 'ERR_MODULE_NOT_FOUND') {
            console.warn(`
The environment variable DEV is set to true, so Ink tried to import \`react-devtools-core\`,
but this failed as it was not installed. Debugging with React Devtools requires it.

To install use this command:

$ npm install --save-dev react-devtools-core
				`.trim() + '\n');
        }
        else {
            // eslint-disable-next-line @typescript-eslint/only-throw-error
            throw error;
        }
    }
}
const diff = (before, after) => {
    if (before === after) {
        return;
    }
    if (!before) {
        return after;
    }
    const changed = {};
    let isChanged = false;
    for (const key of Object.keys(before)) {
        const isDeleted = after ? !Object.hasOwn(after, key) : true;
        if (isDeleted) {
            changed[key] = undefined;
            isChanged = true;
        }
    }
    if (after) {
        for (const key of Object.keys(after)) {
            if (after[key] !== before[key]) {
                changed[key] = after[key];
                isChanged = true;
            }
        }
    }
    return isChanged ? changed : undefined;
};
const cleanupYogaNode = (node) => {
    node?.unsetMeasureFunc();
    node?.freeRecursive();
};
export default createReconciler({
    getRootHostContext: () => ({
        isInsideText: false,
    }),
    prepareForCommit: () => null,
    preparePortalMount: () => null,
    clearContainer: () => false,
    resetAfterCommit(rootNode) {
        if (typeof rootNode.onComputeLayout === 'function') {
            rootNode.onComputeLayout();
        }
        // Since renders are throttled at the instance level and <Static> component children
        // are rendered only once and then get deleted, we need an escape hatch to
        // trigger an immediate render to ensure <Static> children are written to output before they get erased
        if (rootNode.isStaticDirty) {
            rootNode.isStaticDirty = false;
            if (typeof rootNode.onImmediateRender === 'function') {
                rootNode.onImmediateRender();
            }
            return;
        }
        if (typeof rootNode.onRender === 'function') {
            rootNode.onRender();
        }
    },
    getChildHostContext(parentHostContext, type) {
        const previousIsInsideText = parentHostContext.isInsideText;
        const isInsideText = type === 'ink-text' || type === 'ink-virtual-text';
        if (previousIsInsideText === isInsideText) {
            return parentHostContext;
        }
        return { isInsideText };
    },
    shouldSetTextContent: () => false,
    createInstance(originalType, newProps, _root, hostContext) {
        if (hostContext.isInsideText && originalType === 'ink-box') {
            throw new Error(`<Box> can’t be nested inside <Text> component`);
        }
        const type = originalType === 'ink-text' && hostContext.isInsideText
            ? 'ink-virtual-text'
            : originalType;
        const node = createNode(type);
        for (const [key, value] of Object.entries(newProps)) {
            if (key === 'children') {
                continue;
            }
            if (key === 'style') {
                setStyle(node, value);
                if (node.yogaNode) {
                    applyStyles(node.yogaNode, value);
                }
                continue;
            }
            if (key === 'internal_transform') {
                node.internal_transform = value;
                continue;
            }
            if (key === 'internal_static') {
                node.internal_static = true;
                continue;
            }
            setAttribute(node, key, value);
        }
        return node;
    },
    createTextInstance(text, _root, hostContext) {
        if (!hostContext.isInsideText) {
            throw new Error(`Text string "${text}" must be rendered inside <Text> component`);
        }
        return createTextNode(text);
    },
    resetTextContent() { },
    hideTextInstance(node) {
        setTextNodeValue(node, '');
    },
    unhideTextInstance(node, text) {
        setTextNodeValue(node, text);
    },
    getPublicInstance: instance => instance,
    hideInstance(node) {
        node.yogaNode?.setDisplay(Yoga.DISPLAY_NONE);
    },
    unhideInstance(node) {
        node.yogaNode?.setDisplay(Yoga.DISPLAY_FLEX);
    },
    appendInitialChild: appendChildNode,
    appendChild: appendChildNode,
    insertBefore: insertBeforeNode,
    finalizeInitialChildren(node, _type, _props, rootNode) {
        if (node.internal_static) {
            rootNode.isStaticDirty = true;
            // Save reference to <Static> node to skip traversal of entire
            // node tree to find it
            rootNode.staticNode = node;
        }
        return false;
    },
    isPrimaryRenderer: true,
    supportsMutation: true,
    supportsPersistence: false,
    supportsHydration: false,
    scheduleTimeout: setTimeout,
    cancelTimeout: clearTimeout,
    noTimeout: -1,
    getCurrentEventPriority: () => DefaultEventPriority,
    beforeActiveInstanceBlur() { },
    afterActiveInstanceBlur() { },
    detachDeletedInstance() { },
    getInstanceFromNode: () => null,
    prepareScopeUpdate() { },
    getInstanceFromScope: () => null,
    appendChildToContainer: appendChildNode,
    insertInContainerBefore: insertBeforeNode,
    removeChildFromContainer(node, removeNode) {
        removeChildNode(node, removeNode);
        cleanupYogaNode(removeNode.yogaNode);
    },
    prepareUpdate(node, _type, oldProps, newProps, rootNode) {
        if (node.internal_static) {
            rootNode.isStaticDirty = true;
        }
        const props = diff(oldProps, newProps);
        const style = diff(oldProps['style'], newProps['style']);
        if (!props && !style) {
            return null;
        }
        return { props, style };
    },
    commitUpdate(node, { props, style }) {
        if (props) {
            for (const [key, value] of Object.entries(props)) {
                if (key === 'style') {
                    setStyle(node, value);
                    continue;
                }
                if (key === 'internal_transform') {
                    node.internal_transform = value;
                    continue;
                }
                if (key === 'internal_static') {
                    node.internal_static = true;
                    continue;
                }
                setAttribute(node, key, value);
            }
        }
        if (style && node.yogaNode) {
            applyStyles(node.yogaNode, style);
        }
    },
    commitTextUpdate(node, _oldText, newText) {
        setTextNodeValue(node, newText);
    },
    removeChild(node, removeNode) {
        removeChildNode(node, removeNode);
        cleanupYogaNode(removeNode.yogaNode);
    },
});
//# sourceMappingURL=reconciler.js.map