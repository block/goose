"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _yogaLayoutPrebuilt = _interopRequireDefault(require("yoga-layout-prebuilt"));

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

const applyPositionStyles = (node, style) => {
  if (!style.position) {
    node.setPositionType(NaN);
  }

  if (style.position === 'absolute') {
    node.setPositionType(_yogaLayoutPrebuilt.default.POSITION_TYPE_ABSOLUTE);
  }
};

const applyMarginStyles = (node, style) => {
  node.setMargin(_yogaLayoutPrebuilt.default.EDGE_START, style.marginLeft || style.marginX || style.margin || 0);
  node.setMargin(_yogaLayoutPrebuilt.default.EDGE_END, style.marginRight || style.marginX || style.margin || 0);
  node.setMargin(_yogaLayoutPrebuilt.default.EDGE_TOP, style.marginTop || style.marginY || style.margin || 0);
  node.setMargin(_yogaLayoutPrebuilt.default.EDGE_BOTTOM, style.marginBottom || style.marginY || style.margin || 0);
};

const applyPaddingStyles = (node, style) => {
  node.setPadding(_yogaLayoutPrebuilt.default.EDGE_LEFT, style.paddingLeft || style.paddingX || style.padding || 0);
  node.setPadding(_yogaLayoutPrebuilt.default.EDGE_RIGHT, style.paddingRight || style.paddingX || style.padding || 0);
  node.setPadding(_yogaLayoutPrebuilt.default.EDGE_TOP, style.paddingTop || style.paddingY || style.padding || 0);
  node.setPadding(_yogaLayoutPrebuilt.default.EDGE_BOTTOM, style.paddingBottom || style.paddingY || style.padding || 0);
};

const applyFlexStyles = (node, style) => {
  node.setFlexGrow(style.flexGrow || 0);
  node.setFlexShrink(typeof style.flexShrink === 'number' ? style.flexShrink : 1);

  if (style.flexDirection === 'row') {
    node.setFlexDirection(_yogaLayoutPrebuilt.default.FLEX_DIRECTION_ROW);
  }

  if (style.flexDirection === 'row-reverse') {
    node.setFlexDirection(_yogaLayoutPrebuilt.default.FLEX_DIRECTION_ROW_REVERSE);
  }

  if (style.flexDirection === 'column') {
    node.setFlexDirection(_yogaLayoutPrebuilt.default.FLEX_DIRECTION_COLUMN);
  }

  if (style.flexDirection === 'column-reverse') {
    node.setFlexDirection(_yogaLayoutPrebuilt.default.FLEX_DIRECTION_COLUMN_REVERSE);
  }

  if (typeof style.flexBasis === 'number') {
    node.setFlexBasis(style.flexBasis);
  } else if (typeof style.flexBasis === 'string') {
    node.setFlexBasisPercent(parseInt(style.flexBasis, 10));
  } else {
    // This should be replaced with node.setFlexBasisAuto() when new Yoga release is out
    node.setFlexBasis(NaN);
  }

  if (style.alignItems === 'stretch' || !style.alignItems) {
    node.setAlignItems(_yogaLayoutPrebuilt.default.ALIGN_STRETCH);
  }

  if (style.alignItems === 'flex-start') {
    node.setAlignItems(_yogaLayoutPrebuilt.default.ALIGN_FLEX_START);
  }

  if (style.alignItems === 'center') {
    node.setAlignItems(_yogaLayoutPrebuilt.default.ALIGN_CENTER);
  }

  if (style.alignItems === 'flex-end') {
    node.setAlignItems(_yogaLayoutPrebuilt.default.ALIGN_FLEX_END);
  }

  if (style.justifyContent === 'flex-start' || !style.justifyContent) {
    node.setJustifyContent(_yogaLayoutPrebuilt.default.JUSTIFY_FLEX_START);
  }

  if (style.justifyContent === 'center') {
    node.setJustifyContent(_yogaLayoutPrebuilt.default.JUSTIFY_CENTER);
  }

  if (style.justifyContent === 'flex-end') {
    node.setJustifyContent(_yogaLayoutPrebuilt.default.JUSTIFY_FLEX_END);
  }

  if (style.justifyContent === 'space-between') {
    node.setJustifyContent(_yogaLayoutPrebuilt.default.JUSTIFY_SPACE_BETWEEN);
  }

  if (style.justifyContent === 'space-around') {
    node.setJustifyContent(_yogaLayoutPrebuilt.default.JUSTIFY_SPACE_AROUND);
  }
};

const applyDimensionStyles = (node, style) => {
  if (typeof style.width === 'number') {
    node.setWidth(style.width);
  } else if (typeof style.width === 'string') {
    node.setWidthPercent(parseInt(style.width, 10));
  } else {
    node.setWidthAuto();
  }

  if (typeof style.height === 'number') {
    node.setHeight(style.height);
  } else if (typeof style.height === 'string') {
    node.setHeightPercent(parseInt(style.height, 10));
  } else {
    node.setHeightAuto();
  }

  if (typeof style.minWidth === 'string') {
    node.setMinWidthPercent(parseInt(style.minWidth, 10));
  } else {
    node.setMinWidth(style.minWidth || 0);
  }

  if (typeof style.minHeight === 'string') {
    node.setMinHeightPercent(parseInt(style.minHeight, 10));
  } else {
    node.setMinHeight(style.minHeight || 0);
  }
};

var _default = (node, style = {}) => {
  applyPositionStyles(node, style);
  applyMarginStyles(node, style);
  applyPaddingStyles(node, style);
  applyFlexStyles(node, style);
  applyDimensionStyles(node, style);
};

exports.default = _default;