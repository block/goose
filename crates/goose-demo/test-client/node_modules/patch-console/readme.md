# patch-console ![test](https://github.com/vadimdemedes/patch-console/workflows/test/badge.svg)

> Patch console methods to intercept output

## Install

```
$ npm install patch-console
```

## Usage

```js
import patchConsole from 'patch-console';

const restore = patchConsole((stream, data) => {
	// stream = 'stdout'
	// data = "Hello World"
});

console.log('Hello World');

// Restore original methods
restore();
```

## API

### patchConsole(callback)

After this function is called, output from console methods will be intercepted and won't show up in the actual stdout or stderr stream.
To restore original console methods and stop intercepting output, call the function which `patchConsole()` returns.

#### callback

Type: `Function`

Function that will be called when output from one of the console methods is intercepted.
First argument is name of the stream (`"stdout"` or `"stderr"`), second argument is output itself.

## Console methods

This module intercepts the following methods:

- `console.assert()`
- `console.count()`
- `console.countReset()`
- `console.debug()`
- `console.dir()`
- `console.dirxml()`
- `console.error()`
- `console.group()`
- `console.groupCollapsed()`
- `console.groupEnd()`
- `console.info()`
- `console.log()`
- `console.table()`
- `console.time()`
- `console.timeEnd()`
- `console.timeLog()`
- `console.trace()`
- `console.warn()`
