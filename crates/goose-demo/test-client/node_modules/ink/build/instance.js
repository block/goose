"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _react = _interopRequireDefault(require("react"));

var _lodash = _interopRequireDefault(require("lodash.throttle"));

var _autoBind = _interopRequireDefault(require("auto-bind"));

var _logUpdate = _interopRequireDefault(require("log-update"));

var _isCi = _interopRequireDefault(require("is-ci"));

var _signalExit = _interopRequireDefault(require("signal-exit"));

var _ansiEscapes = _interopRequireDefault(require("ansi-escapes"));

var _reconciler = _interopRequireDefault(require("./reconciler"));

var _reconciler2 = _interopRequireDefault(require("./experimental/reconciler"));

var _renderer = _interopRequireDefault(require("./renderer"));

var _renderer2 = _interopRequireDefault(require("./experimental/renderer"));

var dom = _interopRequireWildcard(require("./dom"));

var experimentalDom = _interopRequireWildcard(require("./experimental/dom"));

var _instances = _interopRequireDefault(require("./instances"));

var _App = _interopRequireDefault(require("./components/App"));

function _getRequireWildcardCache() { if (typeof WeakMap !== "function") return null; var cache = new WeakMap(); _getRequireWildcardCache = function () { return cache; }; return cache; }

function _interopRequireWildcard(obj) { if (obj && obj.__esModule) { return obj; } if (obj === null || typeof obj !== "object" && typeof obj !== "function") { return { default: obj }; } var cache = _getRequireWildcardCache(); if (cache && cache.has(obj)) { return cache.get(obj); } var newObj = {}; var hasPropertyDescriptor = Object.defineProperty && Object.getOwnPropertyDescriptor; for (var key in obj) { if (Object.prototype.hasOwnProperty.call(obj, key)) { var desc = hasPropertyDescriptor ? Object.getOwnPropertyDescriptor(obj, key) : null; if (desc && (desc.get || desc.set)) { Object.defineProperty(newObj, key, desc); } else { newObj[key] = obj[key]; } } } newObj.default = obj; if (cache) { cache.set(obj, newObj); } return newObj; }

function _interopRequireDefault(obj) { return obj && obj.__esModule ? obj : { default: obj }; }

class Instance {
  constructor(options) {
    (0, _autoBind.default)(this);
    this.options = options;

    if (options.experimental) {
      this.rootNode = experimentalDom.createNode('root');
      this.rootNode.onRender = options.debug ? this.onRender : (0, _lodash.default)(this.onRender, 16, {
        leading: true,
        trailing: true
      });
      this.rootNode.onImmediateRender = this.onRender;
      this.renderer = (0, _renderer2.default)({
        terminalWidth: options.stdout.columns
      });
    } else {
      this.rootNode = dom.createNode('root');
      this.rootNode.onRender = this.onRender;
      this.renderer = (0, _renderer.default)({
        terminalWidth: options.stdout.columns
      });
    }

    this.log = _logUpdate.default.create(options.stdout);
    this.throttledLog = options.debug ? this.log : (0, _lodash.default)(this.log, {
      leading: true,
      trailing: true
    }); // Ignore last render after unmounting a tree to prevent empty output before exit

    this.isUnmounted = false; // Store last output to only rerender when needed

    this.lastOutput = ''; // This variable is used only in debug mode to store full static output
    // so that it's rerendered every time, not just new static parts, like in non-debug mode

    this.fullStaticOutput = '';

    if (options.experimental) {
      this.container = _reconciler2.default.createContainer(this.rootNode, false, false);
    } else {
      this.container = _reconciler.default.createContainer(this.rootNode, false, false);
    }

    this.exitPromise = new Promise((resolve, reject) => {
      this.resolveExitPromise = resolve;
      this.rejectExitPromise = reject;
    }); // Unmount when process exits

    this.unsubscribeExit = (0, _signalExit.default)(this.unmount, {
      alwaysLast: false
    });
  }

  onRender() {
    if (this.isUnmounted) {
      return;
    }

    const {
      output,
      outputHeight,
      staticOutput
    } = this.renderer(this.rootNode); // If <Static> output isn't empty, it means new children have been added to it

    const hasStaticOutput = staticOutput && staticOutput !== '\n';

    if (this.options.debug) {
      if (hasStaticOutput) {
        this.fullStaticOutput += staticOutput;
      }

      this.options.stdout.write(this.fullStaticOutput + output);
      return;
    }

    if (_isCi.default) {
      if (hasStaticOutput) {
        this.options.stdout.write(staticOutput);
      }

      this.lastOutput = output;
      return;
    }

    if (hasStaticOutput) {
      this.fullStaticOutput += staticOutput;
    }

    if (this.options.experimental && outputHeight >= this.options.stdout.rows) {
      this.options.stdout.write(_ansiEscapes.default.clearTerminal + this.fullStaticOutput + output);
      this.lastOutput = output;
      return;
    } // To ensure static output is cleanly rendered before main output, clear main output first


    if (hasStaticOutput) {
      this.log.clear();
      this.options.stdout.write(staticOutput);
    }

    if (output !== this.lastOutput) {
      if (this.options.experimental) {
        this.throttledLog(output);
      } else {
        this.log(output);
      }
    }
  }

  render(node) {
    const tree = _react.default.createElement(_App.default, {
      stdin: this.options.stdin,
      stdout: this.options.stdout,
      exitOnCtrlC: this.options.exitOnCtrlC,
      onExit: this.unmount
    }, node);

    if (this.options.experimental) {
      _reconciler2.default.updateContainer(tree, this.container);
    } else {
      _reconciler.default.updateContainer(tree, this.container);
    }
  }

  unmount(error) {
    if (this.isUnmounted) {
      return;
    }

    this.onRender();
    this.unsubscribeExit(); // CIs don't handle erasing ansi escapes well, so it's better to
    // only render last frame of non-static output

    if (_isCi.default) {
      this.options.stdout.write(this.lastOutput + '\n');
    } else if (!this.options.debug) {
      this.log.done();
    }

    this.isUnmounted = true;

    if (this.options.experimental) {
      _reconciler2.default.updateContainer(null, this.container);
    } else {
      _reconciler.default.updateContainer(null, this.container);
    }

    _instances.default.delete(this.options.stdout);

    if (error instanceof Error) {
      this.rejectExitPromise(error);
    } else {
      this.resolveExitPromise();
    }
  }

  waitUntilExit() {
    return this.exitPromise;
  }

}

exports.default = Instance;