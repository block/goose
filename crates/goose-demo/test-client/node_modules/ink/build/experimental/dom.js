"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.setTextContent = exports.createTextNode = exports.setAttribute = exports.setStyle = exports.removeChildNode = exports.insertBeforeNode = exports.appendChildNode = exports.createNode = void 0;

var _yogaLayoutPrebuilt = _interopRequireDefault(require("yoga-layout-prebuilt"));

var _measureText = _interopRequireDefault(require("../measure-text"));

var _applyStyle = _interopRequireDefault(require("./apply-style"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

// Helper utilities implementing some common DOM methods to simplify reconciliation code
const createNode = tagName => ({
  nodeName: tagName.toUpperCase(),
  style: {},
  attributes: {},
  childNodes: [],
  parentNode: null,
  textContent: null,
  yogaNode: _yogaLayoutPrebuilt.default.Node.create()
});

exports.createNode = createNode;

const appendChildNode = (node, childNode) => {
  if (childNode.parentNode) {
    removeChildNode(childNode.parentNode, childNode);
  }

  childNode.parentNode = node;
  node.childNodes.push(childNode);
  node.yogaNode.insertChild(childNode.yogaNode, node.yogaNode.getChildCount());
};

exports.appendChildNode = appendChildNode;

const insertBeforeNode = (node, newChildNode, beforeChildNode) => {
  if (newChildNode.parentNode) {
    removeChildNode(newChildNode.parentNode, newChildNode);
  }

  newChildNode.parentNode = node;
  const index = node.childNodes.indexOf(beforeChildNode);

  if (index >= 0) {
    node.childNodes.splice(index, 0, newChildNode);
    node.yogaNode.insertChild(newChildNode.yogaNode, index);
    return;
  }

  node.childNodes.push(newChildNode);
  node.yogaNode.insertChild(newChildNode.yogaNode, node.yogaNode.getChildCount());
};

exports.insertBeforeNode = insertBeforeNode;

const removeChildNode = (node, removeNode) => {
  removeNode.parentNode.yogaNode.removeChild(removeNode.yogaNode);
  removeNode.parentNode = null;
  const index = node.childNodes.indexOf(removeNode);

  if (index >= 0) {
    node.childNodes.splice(index, 1);
  }
};

exports.removeChildNode = removeChildNode;

const setStyle = (node, style) => {
  node.style = style;
  (0, _applyStyle.default)(node.yogaNode, style);
};

exports.setStyle = setStyle;

const setAttribute = (node, key, value) => {
  node.attributes[key] = value;
};

exports.setAttribute = setAttribute;

const createTextNode = text => {
  const node = {
    nodeName: '#text',
    nodeValue: text,
    yogaNode: _yogaLayoutPrebuilt.default.Node.create()
  };
  setTextContent(node, text);
  return node;
};

exports.createTextNode = createTextNode;

const setTextContent = (node, text) => {
  if (typeof text !== 'string') {
    text = String(text);
  }

  let width = 0;
  let height = 0;

  if (text.length > 0) {
    const dimensions = (0, _measureText.default)(text);
    width = dimensions.width;
    height = dimensions.height;
  }

  if (node.nodeName === '#text') {
    node.nodeValue = text;
    node.yogaNode.setWidth(width);
    node.yogaNode.setHeight(height);
  } else {
    node.textContent = text;
    node.yogaNode.setWidth(node.style.width || width);
    node.yogaNode.setHeight(node.style.height || height);
  }
};

exports.setTextContent = setTextContent;