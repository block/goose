"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _scheduler = require("scheduler");

var _reactReconciler = _interopRequireDefault(require("react-reconciler"));

var _dom = require("./dom");

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

const NO_CONTEXT = true;
const hostConfig = {
  schedulePassiveEffects: _scheduler.unstable_scheduleCallback,
  cancelPassiveEffects: _scheduler.unstable_cancelCallback,
  now: Date.now,
  getRootHostContext: () => NO_CONTEXT,
  prepareForCommit: () => {},
  resetAfterCommit: rootNode => {
    // Since renders are throttled at the instance level and <Static> component children
    // are rendered only once and then get deleted, we need an escape hatch to
    // trigger an immediate render to ensure <Static> children are written to output before they get erased
    if (rootNode.isStaticDirty) {
      rootNode.isStaticDirty = false;
      rootNode.onImmediateRender();
      return;
    }

    rootNode.onRender();
  },
  getChildHostContext: () => NO_CONTEXT,
  shouldSetTextContent: (type, props) => {
    return typeof props.children === 'string' || typeof props.children === 'number';
  },
  createInstance: (type, newProps) => {
    const node = (0, _dom.createNode)(type);

    for (const [key, value] of Object.entries(newProps)) {
      if (key === 'children') {
        if (typeof value === 'string' || typeof value === 'number') {
          if (type === 'div') {
            // Text node must be wrapped in another node, so that text can be aligned within container
            const textNode = (0, _dom.createNode)('div');
            (0, _dom.setTextContent)(textNode, value);
            (0, _dom.appendChildNode)(node, textNode);
          }

          if (type === 'span') {
            (0, _dom.setTextContent)(node, value);
          }
        }
      } else if (key === 'style') {
        (0, _dom.setStyle)(node, value);
      } else if (key === 'unstable__transformChildren') {
        node.unstable__transformChildren = value; // eslint-disable-line camelcase
      } else if (key === 'unstable__static') {
        node.unstable__static = true; // eslint-disable-line camelcase
      } else {
        (0, _dom.setAttribute)(node, key, value);
      }
    }

    return node;
  },
  createTextInstance: _dom.createTextNode,
  resetTextContent: node => {
    if (node.textContent) {
      node.textContent = '';
    }

    if (node.childNodes.length > 0) {
      for (const childNode of node.childNodes) {
        (0, _dom.removeChildNode)(node, childNode);
      }
    }
  },
  getPublicInstance: instance => instance,
  appendInitialChild: _dom.appendChildNode,
  appendChild: _dom.appendChildNode,
  insertBefore: _dom.insertBeforeNode,
  finalizeInitialChildren: (node, type, props, rootNode) => {
    if (node.unstable__static) {
      rootNode.isStaticDirty = true;
    }
  },
  supportsMutation: true,
  appendChildToContainer: _dom.appendChildNode,
  insertInContainerBefore: _dom.insertBeforeNode,
  removeChildFromContainer: _dom.removeChildNode,
  prepareUpdate: (node, type, oldProps, newProps, rootNode) => {
    if (node.unstable__static) {
      rootNode.isStaticDirty = true;
    }

    return true;
  },
  commitUpdate: (node, updatePayload, type, oldProps, newProps) => {
    for (const [key, value] of Object.entries(newProps)) {
      if (key === 'children') {
        if (typeof value === 'string' || typeof value === 'number') {
          if (type === 'div') {
            // Text node must be wrapped in another node, so that text can be aligned within container
            // If there's no such node, a new one must be created
            if (node.childNodes.length === 0) {
              const textNode = (0, _dom.createNode)('div');
              (0, _dom.setTextContent)(textNode, value);
              (0, _dom.appendChildNode)(node, textNode);
            } else {
              (0, _dom.setTextContent)(node.childNodes[0], value);
            }
          }

          if (type === 'span') {
            (0, _dom.setTextContent)(node, value);
          }
        }
      } else if (key === 'style') {
        (0, _dom.setStyle)(node, value);
      } else if (key === 'unstable__transformChildren') {
        node.unstable__transformChildren = value; // eslint-disable-line camelcase
      } else if (key === 'unstable__static') {
        node.unstable__static = true; // eslint-disable-line camelcase
      } else {
        (0, _dom.setAttribute)(node, key, value);
      }
    }
  },
  commitTextUpdate: (node, oldText, newText) => {
    (0, _dom.setTextContent)(node, newText);
  },
  removeChild: _dom.removeChildNode
};

var _default = (0, _reactReconciler.default)(hostConfig); // eslint-disable-line new-cap


exports.default = _default;