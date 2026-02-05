# code-excerpt ![test](https://github.com/vadimdemedes/code-excerpt/workflows/test/badge.svg)

> Extract code excerpts

## Install

```
$ npm install --save code-excerpt
```

## Usage

```js
import codeExcerpt from 'code-excerpt';

const source = `
'use strict';

function someFunc() {}

module.exports = () => {
	const a = 1;
	const b = 2;
	const c = 3;

	someFunc();
};
`.trim();

const excerpt = codeExcerpt(source, 5);
//=> [
//	{line: 2, value: ''},
//	{line: 3, value: 'function someFunc() {}'},
//	{line: 4, value: ''},
//	{line: 5, value: 'module.exports = () => {'},
//	{line: 6, value: '  const a = 1;'},
//	{line: 7, value: '  const b = 2;'},
//	{line: 8, value: '  const c = 3;'}
// ]
```

## API

### codeExcerpt(source, line, [options])

#### source

Type: `string`

Source code.

#### line

Type: `number`

Line number to extract excerpt for.

#### options

##### around

Type: `number`<br>
Default: `3`

Number of surrounding lines to extract.
