"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _react = require("react");

var _ = require("..");

var _default = inputHandler => {
  const {
    stdin,
    setRawMode
  } = (0, _react.useContext)(_.StdinContext);
  (0, _react.useLayoutEffect)(() => {
    setRawMode(true);
    return () => setRawMode(false);
  }, [setRawMode]);
  (0, _react.useLayoutEffect)(() => {
    const handleData = data => {
      const input = String(data);
      const meta = {
        up: input === '\u001B[A',
        down: input === '\u001B[B',
        left: input === '\u001B[D',
        right: input === '\u001B[C'
      };
      inputHandler(input, meta);
    };

    stdin.on('data', handleData);
    return () => stdin.off('data', handleData);
  }, [stdin, inputHandler]);
};

exports.default = _default;