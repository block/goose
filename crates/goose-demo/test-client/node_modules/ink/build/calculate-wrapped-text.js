"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _measureText = _interopRequireDefault(require("./measure-text"));

var _wrapText = _interopRequireDefault(require("./wrap-text"));

var _getMaxWidth = _interopRequireDefault(require("./get-max-width"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

// Since we need to know the width of text container to wrap text, we have to calculate layout twice
// This function is executed after first layout calculation to reassign width and height of text nodes
const calculateWrappedText = node => {
  if (node.textContent && typeof node.parentNode.style.textWrap === 'string') {
    const {
      yogaNode
    } = node;
    const parentYogaNode = node.parentNode.yogaNode;
    const maxWidth = (0, _getMaxWidth.default)(parentYogaNode);
    const currentWidth = yogaNode.getComputedWidth();

    if (currentWidth > maxWidth) {
      const {
        textWrap
      } = node.parentNode.style;
      const wrappedText = (0, _wrapText.default)(node.textContent, maxWidth, {
        textWrap
      });
      const {
        width,
        height
      } = (0, _measureText.default)(wrappedText);
      yogaNode.setWidth(width);
      yogaNode.setHeight(height);
    }

    return;
  }

  if (Array.isArray(node.childNodes) && node.childNodes.length > 0) {
    for (const childNode of node.childNodes) {
      calculateWrappedText(childNode);
    }
  }
};

var _default = calculateWrappedText;
exports.default = _default;