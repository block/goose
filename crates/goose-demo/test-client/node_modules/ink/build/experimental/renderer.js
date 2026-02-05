"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _yogaLayoutPrebuilt = _interopRequireDefault(require("yoga-layout-prebuilt"));

var _renderNodeToOutput = _interopRequireDefault(require("../render-node-to-output"));

var _calculateWrappedText = _interopRequireDefault(require("../calculate-wrapped-text"));

var _output = _interopRequireDefault(require("./output"));

var _dom = require("./dom");

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

// Since <Static> components can be placed anywhere in the tree, this helper finds and returns them
const findStaticNode = node => {
  if (node.unstable__static) {
    return node;
  }

  for (const childNode of node.childNodes) {
    if (childNode.unstable__static) {
      return childNode;
    }

    if (Array.isArray(childNode.childNodes) && childNode.childNodes.length > 0) {
      return findStaticNode(childNode);
    }
  }
};

var _default = ({
  terminalWidth = 100
}) => {
  return node => {
    (0, _dom.setStyle)(node, {
      width: terminalWidth
    });
    node.yogaNode.calculateLayout(_yogaLayoutPrebuilt.default.UNDEFINED, _yogaLayoutPrebuilt.default.UNDEFINED, _yogaLayoutPrebuilt.default.DIRECTION_LTR);
    (0, _calculateWrappedText.default)(node);
    node.yogaNode.calculateLayout(_yogaLayoutPrebuilt.default.UNDEFINED, _yogaLayoutPrebuilt.default.UNDEFINED, _yogaLayoutPrebuilt.default.DIRECTION_LTR);
    const output = new _output.default({
      width: node.yogaNode.getComputedWidth(),
      height: node.yogaNode.getComputedHeight()
    });
    (0, _renderNodeToOutput.default)(node, output, {
      skipStaticElements: true
    });
    const staticNode = findStaticNode(node);
    let staticOutput;

    if (staticNode) {
      staticOutput = new _output.default({
        width: staticNode.yogaNode.getComputedWidth(),
        height: staticNode.yogaNode.getComputedHeight()
      });
      (0, _renderNodeToOutput.default)(staticNode, staticOutput, {
        skipStaticElements: false
      });
    }

    const {
      output: generatedOutput,
      height: outputHeight
    } = output.get();
    return {
      output: generatedOutput,
      outputHeight,
      // Newline at the end is needed, because static output doesn't have one, so
      // interactive output will override last line of static output
      staticOutput: staticOutput ? `${staticOutput.get().output}\n` : undefined
    };
  };
};

exports.default = _default;